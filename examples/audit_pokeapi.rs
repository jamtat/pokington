//! Full cached `PokéAPI` feasibility audit. Does not select or adopt this provider.
use http_body_util::{BodyExt, Limited};
use pokington::source::http_client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
};

type Failure = Box<dyn Error + Send + Sync>;
const API: &str = "https://pokeapi.co/api/v2/";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Resource {
    name: String,
    url: String,
}
#[derive(Deserialize)]
struct Page {
    count: usize,
    next: Option<String>,
    results: Vec<Resource>,
}

fn cache_path(dir: &Path, url: &str) -> Result<PathBuf, Failure> {
    let rest = url.strip_prefix(API).ok_or("foreign API resource URL")?;
    if rest.contains("..")
        || !rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-/?=&".contains(&b))
    {
        return Err("invalid API resource URL".into());
    }
    Ok(dir.join(format!("{}.json", rest.replace(['/', '?', '&', '='], "_"))))
}

async fn get(
    client: &wreq::Client,
    dir: &Path,
    url: &str,
    offline: bool,
) -> Result<Value, Failure> {
    let path = cache_path(dir, url)?;
    if path.exists() {
        return Ok(serde_json::from_slice(&fs::read(path)?)?);
    }
    if offline {
        return Err(format!("uncached URL: {url}").into());
    }
    let response = client.get(url).send().await?.error_for_status()?;
    if !response.uri().to_string().starts_with(API) {
        return Err("foreign API redirect".into());
    }
    let bytes = Limited::new(response, 16 * 1024 * 1024)
        .collect()
        .await?
        .to_bytes();
    let value = serde_json::from_slice(&bytes)?;
    fs::write(path, bytes)?;
    Ok(value)
}

async fn list(
    client: &wreq::Client,
    dir: &Path,
    endpoint: &str,
    offline: bool,
) -> Result<Vec<Resource>, Failure> {
    let mut next = Some(format!("{API}{endpoint}/?limit=200"));
    let mut visited = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut urls = BTreeSet::new();
    let mut expected = None;
    let mut resources = Vec::new();
    while let Some(url) = next {
        if !visited.insert(url.clone()) {
            return Err("pagination loop".into());
        }
        let page: Page = serde_json::from_value(get(client, dir, &url, offline).await?)?;
        if expected.is_some_and(|count| count != page.count) {
            return Err("source count changed during pagination".into());
        }
        expected = Some(page.count);
        for resource in page.results {
            if !names.insert(resource.name.clone()) || !urls.insert(resource.url.clone()) {
                return Err("duplicate listed resource".into());
            }
            if !resource.url.starts_with(&format!("{API}{endpoint}/")) {
                return Err("wrong endpoint listed".into());
            }
            resources.push(resource);
        }
        next = page.next;
    }
    if Some(resources.len()) != expected {
        return Err("partial pagination".into());
    }
    Ok(resources)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, Failure> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {key}"))?
        .as_array()
        .ok_or_else(|| format!("missing array {key}").into())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, Failure> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {key}"))?
        .as_str()
        .ok_or_else(|| format!("missing string {key}").into())
}
fn resource_url<'a>(value: &'a Value, key: &str) -> Result<&'a str, Failure> {
    string(
        value
            .get(key)
            .ok_or_else(|| format!("missing field {key}"))?,
        "url",
    )
}
fn english(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry
                .get("language")
                .and_then(|language| language.get("name"))
                .and_then(Value::as_str)
                == Some("en")
        })
        .filter_map(|entry| entry.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect()
}
fn types(value: &Value) -> Result<Vec<String>, Failure> {
    let mut entries: Vec<_> = array(value, "types")?
        .iter()
        .map(|entry| {
            Ok((
                entry
                    .get("slot")
                    .and_then(Value::as_u64)
                    .ok_or("missing type slot")?,
                string(entry.get("type").ok_or("missing type object")?, "name")?.to_owned(),
            ))
        })
        .collect::<Result<_, Failure>>()?;
    entries.sort();
    if !(1..=2).contains(&entries.len())
        || entries.iter().enumerate().any(|(i, (slot, _))| {
            u64::try_from(i).ok().and_then(|index| index.checked_add(1)) != Some(*slot)
        })
    {
        return Err("invalid ordered type slots".into());
    }
    Ok(entries.into_iter().map(|(_, name)| name).collect())
}

fn relative_key(species_slug: &str, form_slug: &str) -> Result<String, Failure> {
    if form_slug == species_slug {
        return Ok("default".to_owned());
    }
    let suffix = form_slug
        .strip_prefix(&format!("{species_slug}-"))
        .ok_or("form name does not start with its source species slug")?;
    if suffix.is_empty() {
        return Err("empty alternate form suffix".into());
    }
    Ok(suffix.replace('-', " "))
}

struct Coverage {
    problems: Vec<String>,
    pokemon_refs: BTreeSet<String>,
    form_refs: BTreeSet<String>,
    national: BTreeSet<u64>,
    original_forms: BTreeMap<String, String>,
    english_names: BTreeMap<String, Vec<String>>,
}

fn inspect_coverage(
    lists: &BTreeMap<&str, Vec<Resource>>,
    data: &BTreeMap<String, Value>,
) -> Result<Coverage, Failure> {
    let mut coverage = Coverage {
        problems: Vec::new(),
        pokemon_refs: BTreeSet::new(),
        form_refs: BTreeSet::new(),
        national: BTreeSet::new(),
        original_forms: BTreeMap::new(),
        english_names: BTreeMap::new(),
    };
    inspect_species_coverage(lists, data, &mut coverage)?;
    inspect_pokemon_coverage(lists, data, &mut coverage)?;
    Ok(coverage)
}

fn inspect_species_coverage(
    lists: &BTreeMap<&str, Vec<Resource>>,
    data: &BTreeMap<String, Value>,
    coverage: &mut Coverage,
) -> Result<(), Failure> {
    let species_resources = lists
        .get("pokemon-species")
        .ok_or("missing species listing")?;
    for resource in species_resources {
        let species = data.get(&resource.url).ok_or("species details missing")?;
        let id = species
            .get("id")
            .and_then(Value::as_u64)
            .ok_or("species ID")?;
        let names = english(species, "names");
        if names.len() != 1 {
            coverage.problems.push(format!(
                "{}: expected one English species name, found{}",
                resource.name,
                names.len()
            ));
        }
        coverage.english_names.insert(resource.url.clone(), names);
        let numbers: Vec<_> = array(species, "pokedex_numbers")?
            .iter()
            .filter(|number| {
                number
                    .get("pokedex")
                    .and_then(|pokedex| pokedex.get("name"))
                    .and_then(Value::as_str)
                    == Some("national")
            })
            .collect();
        let national_number = numbers
            .first()
            .and_then(|number| number.get("entry_number"))
            .and_then(Value::as_u64);
        if numbers.len() != 1 || national_number != Some(id) || !coverage.national.insert(id) {
            coverage
                .problems
                .push(format!("{}: inconsistent National number", resource.name));
        }
        let mut defaults = Vec::new();
        for variety in array(species, "varieties")? {
            let url = resource_url(variety, "pokemon")?;
            if !coverage.pokemon_refs.insert(url.to_owned()) {
                coverage
                    .problems
                    .push(format!("duplicate variety reference: {url}"));
            }
            let Some(pokemon) = data.get(url) else {
                coverage.problems.push(format!("unlisted variety: {url}"));
                continue;
            };
            if resource_url(pokemon, "species")? != resource.url {
                coverage
                    .problems
                    .push(format!("wrong species backlink: {url}"));
            }
            if variety.get("is_default").and_then(Value::as_bool) == Some(true) {
                defaults.push(url.to_owned());
            }
            if variety.get("is_default") != pokemon.get("is_default") {
                coverage
                    .problems
                    .push(format!("default flag mismatch: {url}"));
            }
        }
        if defaults.len() == 1 {
            if let Some(default) = defaults.first() {
                coverage
                    .original_forms
                    .insert(resource.url.clone(), default.clone());
            }
        } else {
            coverage.problems.push(format!(
                "{}: {} default varieties",
                resource.name,
                defaults.len()
            ));
        }
    }
    Ok(())
}

fn inspect_pokemon_coverage(
    lists: &BTreeMap<&str, Vec<Resource>>,
    data: &BTreeMap<String, Value>,
    coverage: &mut Coverage,
) -> Result<(), Failure> {
    let pokemon_resources = lists.get("pokemon").ok_or("missing Pokémon listing")?;
    for resource in pokemon_resources {
        let pokemon = data.get(&resource.url).ok_or("Pokémon details missing")?;
        if let Err(error) = types(pokemon) {
            coverage
                .problems
                .push(format!("{}: {error}", resource.name));
        }
        let mut defaults = 0_usize;
        for form_ref in array(pokemon, "forms")? {
            let url = string(form_ref, "url")?;
            if !coverage.form_refs.insert(url.to_owned()) {
                coverage
                    .problems
                    .push(format!("duplicate form reference: {url}"));
            }
            let Some(form) = data.get(url) else {
                coverage.problems.push(format!("unlisted form: {url}"));
                continue;
            };
            if resource_url(form, "pokemon")? != resource.url {
                coverage
                    .problems
                    .push(format!("wrong Pokémon backlink: {url}"));
            }
            if form.get("is_default").and_then(Value::as_bool) == Some(true) {
                defaults = defaults.saturating_add(1);
            }
        }
        if defaults != 1 {
            coverage
                .problems
                .push(format!("{}: {defaults} default forms", resource.name));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty()
        || args.len() > 2
        || (args.len() == 2 && args.get(1).is_none_or(|arg| arg != "--offline"))
    {
        return Err("usage: audit_pokeapi CACHE_DIRECTORY [--offline]".into());
    }
    let cache_dir = args.first().ok_or("missing cache directory")?;
    let dir = Path::new(cache_dir);
    fs::create_dir_all(dir)?;
    let offline = args.len() == 2;
    let client = http_client()?;
    let mut lists = BTreeMap::new();
    for endpoint in ["pokemon-species", "pokemon", "pokemon-form"] {
        let resources = list(&client, dir, endpoint, offline).await?;
        println!(
            "{endpoint}: {} listed resources; all pagination counts agree",
            resources.len()
        );
        lists.insert(endpoint, resources);
    }
    let mut data: BTreeMap<String, Value> = BTreeMap::new();
    for (endpoint, resources) in &lists {
        for batch in resources.chunks(4) {
            let mut tasks = tokio::task::JoinSet::new();
            for resource in batch {
                let client = client.clone();
                let dir = dir.to_owned();
                let resource = resource.clone();
                tasks.spawn(async move {
                    let value = get(&client, &dir, &resource.url, offline).await?;
                    if value.get("name") != Some(&Value::String(resource.name.clone())) {
                        return Err::<_, Failure>("listed/detail name mismatch".into());
                    }
                    let id = value
                        .get("id")
                        .and_then(Value::as_u64)
                        .ok_or("missing detail ID")?;
                    if !resource.url.ends_with(&format!("/{id}/")) {
                        return Err("listed/detail ID mismatch".into());
                    }
                    Ok((resource.url, value))
                });
            }
            while let Some(result) = tasks.join_next().await {
                let (url, value) = result??;
                data.insert(url, value);
            }
            if data.len().is_multiple_of(200) {
                println!(
                    "cached and parsed {} total resources ({endpoint})",
                    data.len()
                );
            }
        }
        println!("{endpoint}: {} details fetched/cached", resources.len());
    }
    audit(&lists, &data, dir)?;
    Ok(())
}

fn audit(
    lists: &BTreeMap<&str, Vec<Resource>>,
    data: &BTreeMap<String, Value>,
    dir: &Path,
) -> Result<(), Failure> {
    let coverage = inspect_coverage(lists, data)?;
    let mut problems = coverage.problems;
    let pokemon_refs = coverage.pokemon_refs;
    let form_refs = coverage.form_refs;
    let national = coverage.national;
    let original_forms = coverage.original_forms;
    let english_names = coverage.english_names;
    let expected_pokemon: BTreeSet<_> = lists["pokemon"].iter().map(|r| r.url.clone()).collect();
    let expected_forms: BTreeSet<_> = lists["pokemon-form"]
        .iter()
        .map(|r| r.url.clone())
        .collect();
    for url in expected_pokemon.symmetric_difference(&pokemon_refs) {
        problems.push(format!("species/variety universe mismatch: {url}"));
    }
    for url in expected_forms.symmetric_difference(&form_refs) {
        problems.push(format!("Pokémon/form universe mismatch: {url}"));
    }
    let mut forms = Vec::new();
    let mut keys = BTreeSet::new();
    let mut relative_keys = BTreeSet::new();
    for resource in &lists["pokemon-form"] {
        let form = &data[&resource.url];
        let pokemon_url = resource_url(form, "pokemon")?;
        let pokemon = data
            .get(pokemon_url)
            .ok_or("form's Pokémon missing from listing")?;
        let species_url = resource_url(pokemon, "species")?;
        let species = data.get(species_url).ok_or("unlisted species")?;
        let relative_key = relative_key(string(species, "name")?, &resource.name)?;
        if !relative_keys.insert((species_url.to_owned(), relative_key.clone())) {
            return Err(format!("duplicate source-relative key: {}", resource.name).into());
        }
        let is_original = form.get("is_default").and_then(Value::as_bool) == Some(true)
            && original_forms
                .get(species_url)
                .is_some_and(|url| url == pokemon_url);
        let names = english(form, "form_names");
        let full_names = english(form, "names");
        let raw_key = if is_original {
            "original".to_owned()
        } else if names.len() == 1 && names.first().is_some_and(|name| !name.is_empty()) {
            names.first().cloned().ok_or("missing English form name")?
        } else {
            string(form, "form_name")?.replace('-', " ")
        };
        let key = raw_key
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if key.is_empty() || !keys.insert((species_url.to_owned(), key.clone())) {
            problems.push(format!(
                "missing/duplicate descriptive key: {} -> {key}",
                resource.name
            ));
        }
        let form_types = match types(form) {
            Ok(types) => types,
            Err(error) => {
                problems.push(format!("{} form types: {error}", resource.name));
                Vec::new()
            }
        };
        let official = form
            .pointer("/sprites/other/official-artwork/front_default")
            .cloned()
            .unwrap_or(Value::Null);
        let home = form
            .pointer("/sprites/other/home/front_default")
            .cloned()
            .unwrap_or(Value::Null);
        let sprite = form
            .pointer("/sprites/front_default")
            .cloned()
            .unwrap_or(Value::Null);
        forms.push(json!({"id":form["id"],"name":resource.name,"url":resource.url,"pokemon":pokemon_url,"species":species_url,"species_name":english_names.get(species_url),"is_original":is_original,"is_default":form["is_default"],"is_mega":form["is_mega"],"form_name":form["form_name"],"english_form_names":names,"english_full_names":full_names,"localized_candidate_key":key,"relative_key":relative_key,"types":form_types,"official_artwork":official,"home":home,"form_sprite":sprite}));
    }
    let summary = json!({"species":lists["pokemon-species"].len(),"pokemon":lists["pokemon"].len(),"forms":forms.len(),"national_min":national.first(),"national_max":national.last(),"official_artwork_declared":forms.iter().filter(|f|f["official_artwork"].is_string()).count(),"home_declared":forms.iter().filter(|f|f["home"].is_string()).count(),"form_sprite_declared":forms.iter().filter(|f|f["form_sprite"].is_string()).count(),"no_declared_image":forms.iter().filter(|f|!["official_artwork","home","form_sprite"].iter().any(|key|f[key].is_string())).map(|f|f["name"].clone()).collect::<Vec<_>>(),"problems":problems});
    fs::write(
        dir.join("form-audit.json"),
        serde_json::to_vec_pretty(&forms)?,
    )?;
    fs::write(
        dir.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::relative_key;

    #[test]
    fn descriptive_keys_preserve_cross_species_filters_and_original_colour() {
        assert_eq!(
            relative_key("charizard", "charizard-mega-x").unwrap(),
            "mega x"
        );
        assert_eq!(relative_key("mewtwo", "mewtwo-mega-x").unwrap(), "mega x");
        assert_eq!(relative_key("magearna", "magearna").unwrap(), "default");
        assert_eq!(
            relative_key("magearna", "magearna-original").unwrap(),
            "original"
        );
        assert_eq!(
            relative_key("urshifu", "urshifu-rapid-strike-gmax").unwrap(),
            "rapid strike gmax"
        );
        assert!(relative_key("charizard", "mewtwo-mega-x").is_err());
        assert!(relative_key("charizard", "charizard-").is_err());
    }
}
