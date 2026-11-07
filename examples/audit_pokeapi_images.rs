//! Fetch and validate only image URLs declared by the evaluated PokéAPI form resources.
use http_body_util::{BodyExt, Limited};
use pokington::source::http_client;
use serde_json::{Value, json};
use std::{error::Error, fs, io::Cursor, path::Path};

type Failure = Box<dyn Error + Send + Sync>;

fn inspect(bytes: &[u8]) -> Result<Value, Failure> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.ignore_checksums(false);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let size = reader.output_buffer_size().ok_or("PNG too large")?;
    if size > 64 * 1024 * 1024 {
        return Err("PNG decode limit".into());
    }
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    reader.finish()?;
    let stride = match info.color_type {
        png::ColorType::Rgba => Some(4),
        png::ColorType::GrayscaleAlpha => Some(2),
        _ => None,
    };
    let transparent = stride
        .map(|s| {
            pixels[..info.buffer_size()]
                .chunks_exact(s)
                .filter(|p| p[s - 1] == 0)
                .count()
        })
        .unwrap_or(0);
    Ok(
        json!({"bytes":bytes.len(),"width":info.width,"height":info.height,"color":format!("{:?}",info.color_type),"transparent_pixels":transparent,"full_png_checks":true}),
    )
}

#[tokio::main]
async fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: audit_pokeapi_images FORM_AUDIT_JSON OUTPUT_DIR".into());
    }
    let forms: Vec<Value> = serde_json::from_slice(&fs::read(&args[0])?)?;
    let dir = Path::new(&args[1]);
    fs::create_dir_all(dir)?;
    let client = http_client()?;
    let mut results = Vec::new();
    for batch in forms.chunks(4) {
        let mut tasks = tokio::task::JoinSet::new();
        for form in batch {
            let form = form.clone();
            let client = client.clone();
            let dir = dir.to_owned();
            tasks.spawn(async move{
                let id=form["id"].as_u64().ok_or("form ID")?;let mut attempts=Vec::new();let mut selected=None;
                for tier in ["official_artwork","home","form_sprite"]{
                    let Some(url)=form[tier].as_str()else{continue;};
                    if !url.starts_with("https://raw.githubusercontent.com/PokeAPI/sprites/master/")||!url.ends_with(".png") {return Err::<_,Failure>("unexpected declared image URL".into());}
                    let path=dir.join(format!("{id}-{tier}.response"));let meta=dir.join(format!("{id}-{tier}.json"));
                    let (bytes,status)=if meta.exists(){let info:Value=serde_json::from_slice(&fs::read(&meta)?)?;(fs::read(&path)?,info["status"].as_u64().ok_or("cached status")?)}else{
                        let response=client.get(url).send().await?;let status=response.status().as_u16();let final_url=response.uri().to_string();
                        let bytes=Limited::new(response,16*1024*1024).collect().await?.to_bytes().to_vec();
                        fs::write(&path,&bytes)?;fs::write(&meta,serde_json::to_vec(&json!({"url":url,"final_url":final_url,"status":status,"unix_time":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()}))?)?;
                        (bytes,u64::from(status))
                    };
                    if status!=200{attempts.push(json!({"tier":tier,"url":url,"status":status}));continue;}
                    match inspect(&bytes){Ok(info)=>{attempts.push(json!({"tier":tier,"url":url,"status":status,"png":info}));selected=Some(tier);break;},Err(error)=>attempts.push(json!({"tier":tier,"url":url,"status":status,"decode_error":error.to_string()}))}
                }
                Ok(json!({"id":id,"name":form["name"],"selected":selected,"attempts":attempts}))
            });
        }
        while let Some(result) = tasks.join_next().await {
            results.push(result??);
        }
        if results.len() % 100 == 0 || results.len() == forms.len() {
            println!("image candidates checked {}/{}", results.len(), forms.len());
        }
        fs::write(
            dir.join("results.json"),
            serde_json::to_vec_pretty(&results)?,
        )?;
    }
    let missing: Vec<_> = results
        .iter()
        .filter(|r| r["selected"].is_null())
        .map(|r| r["name"].clone())
        .collect();
    println!(
        "{} forms evaluated; {} lack a fetched/decoded candidate: {}",
        results.len(),
        missing.len(),
        serde_json::to_string(&missing)?
    );
    Ok(())
}
