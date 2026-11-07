use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use pokington::source::{http_client_builder, read_limited};

async fn response(raw: Vec<u8>) -> Result<(reqwest::Response, String), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let server = thread::spawn(move || -> io::Result<String> {
        let (mut socket, _) = listener.accept()?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut request = String::new();
        let mut reader = BufReader::new(&socket);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            request.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        socket.write_all(&raw)?;
        Ok(request)
    });
    let response = http_client_builder()
        .https_only(false)
        .build()?
        .get(format!("http://{address}/"))
        .send()
        .await?;
    Ok((
        response,
        server
            .join()
            .map_err(|_| io::Error::other("HTTP fixture thread panicked"))??,
    ))
}

#[tokio::test]
async fn body_limits_cover_fixed_chunked_and_close_delimited_responses() {
    for raw in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabcdef".as_slice(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nabcdef",
    ] {
        let (res, _) = response(raw.to_vec()).await.unwrap();
        assert_eq!(read_limited(res, 6).await.unwrap(), b"abcdef");
        let (res, _) = response(raw.to_vec()).await.unwrap();
        assert!(
            read_limited(res, 5)
                .await
                .unwrap_err()
                .0
                .contains("5 bytes")
        );
    }
}

#[tokio::test]
async fn truncated_response_is_an_error_and_an_empty_body_respects_zero_limit() {
    let (res, _) = response(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nabcdef".to_vec())
        .await
        .unwrap();
    assert!(read_limited(res, 9).await.is_err());
    let (res, _) = response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec())
        .await
        .unwrap();
    assert_eq!(read_limited(res, 0).await.unwrap(), Vec::<u8>::new());
}

// Independently encoded fixtures for b"compressed response ".repeat(64), generated with
// flate2 1.1.9 (GzEncoder/ZlibEncoder), brotli 9.1.1 (quality 5), zstd 0.14.3 (level 3).
#[tokio::test]
async fn all_compression_formats_are_advertised_decoded_and_limited_after_expansion() {
    let expected = b"compressed response ".repeat(64);
    let encodings: [(&str, &[u8]); 4] = [
        (
            "gzip",
            &[
                31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 237, 200, 177, 9, 0, 64, 8, 3, 192, 85, 220,
                205, 79, 249, 42, 102, 127, 208, 61, 76, 117, 112, 158, 191, 26, 36, 158, 45, 149,
                65, 152, 235, 116, 58, 221, 129, 27, 162, 96, 225, 9, 0, 5, 0, 0,
            ],
        ),
        (
            "deflate",
            &[
                120, 156, 237, 200, 177, 9, 0, 64, 8, 3, 192, 85, 220, 205, 79, 249, 42, 102, 127,
                208, 61, 76, 117, 112, 158, 191, 26, 36, 158, 45, 149, 65, 152, 235, 116, 58, 221,
                129, 27, 124, 16, 249, 16,
            ],
        ),
        (
            "br",
            &[
                27, 255, 4, 0, 4, 28, 114, 164, 103, 127, 97, 43, 55, 241, 196, 177, 164, 176, 140,
                95, 184, 134, 201, 166, 224,
            ],
        ),
        (
            "zstd",
            &[
                40, 181, 47, 253, 0, 88, 229, 0, 0, 160, 99, 111, 109, 112, 114, 101, 115, 115,
                101, 100, 32, 114, 101, 115, 112, 111, 110, 115, 101, 32, 1, 0, 210, 185, 127, 50,
                1,
            ],
        ),
    ];
    for (encoding, body) in encodings {
        assert!(body.len() < expected.len());
        let mut raw = format!(
            "HTTP/1.1 200 OK\r\nContent-Encoding: {encoding}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        raw.extend_from_slice(body);
        let (res, request) = response(raw.clone()).await.unwrap();
        let lower = request.to_ascii_lowercase();
        let accepted = lower
            .lines()
            .find_map(|line| line.strip_prefix("accept-encoding: "))
            .unwrap();
        for supported in ["gzip", "br", "deflate", "zstd"] {
            assert!(accepted.split(',').any(|value| value.trim() == supported));
        }
        assert_eq!(read_limited(res, expected.len()).await.unwrap(), expected);
        let (res, _) = response(raw).await.unwrap();
        assert!(
            read_limited(res, expected.len() - 1)
                .await
                .unwrap_err()
                .0
                .contains("exceeds"),
            "{encoding} must be limited by its decompressed size"
        );
    }
}
