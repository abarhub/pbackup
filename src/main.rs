use reqwest;
use reqwest::Client;

use reqwest::header::USER_AGENT;
use reqwest::Error;
use serde::{Deserialize, Serialize};

use serde_json::Value;

use std::{env, fs};

#[derive(Debug, Deserialize)]
struct Config {
    //app_name: String,
    //debug: bool,
    //port: u16,
    url: String,
    consumer_key: String,
    access_token: String,
    repertoire: String,
}

#[derive(Serialize,Deserialize, Debug)]
struct Parameters {
    consumer_key: String,
    access_token: String,
    detailType: String,
    count: u64,
    offset: u64,
    total: u8,
}

#[derive(Deserialize, Debug)]
struct User {
    login: String,
    id: u32,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // let args: Vec<String> = env::args().collect();
    // if args.len() < 2 {
    //     eprintln!("Usage: {} <config_file>", args[0]);
    //     std::process::exit(1);
    // }
    // let config_path = &args[1];
    //
    // // Lire le contenu du fichier
    // let config_content = fs::read_to_string(config_path)?;
    //
    // // Parser le fichier TOML
    // let config: Config = toml::from_str(&config_content)?;
    //
    // // Afficher la config chargée
    // println!("Configuration chargée : {:?}", config);

    let config_or_err = get_config();

    let config: Config;
    match (config_or_err) {
        Ok(valeur) => {
            println!("Résultat : {:?}", valeur);
            config = valeur
        }
        Err(erreur) => {
            println!("Erreur : {}", erreur);
            std::process::exit(1);
        }
    }
    println!("Configuration chargée : {:?}", config);

    let mut request_url = format!(
        "https://api.github.com/repos/{owner}/{repo}/stargazers",
        //config.url,
        owner = "rust-lang-nursery",
        repo = "rust-cookbook"
    );
    request_url = config.url;
    println!("{}", request_url);

    let client = reqwest::Client::new();
    // let response = client
    //     .get(request_url)
    //     .header(USER_AGENT, "rust-web-api-client") // gh api requires a user-agent header
    //     .send()
    //     .await?;

    //let json_data = r#"{"title":"Problems during installation","status":"todo","priority":"medium","label":"bug"}"#;
    //let json_data = "{\"title":"Problems during installation","status":"todo","priority":"medium","label":"bug"}"#;

    let param=Parameters {
        consumer_key: config.consumer_key,
        access_token: config.access_token,
        detailType: "complete".parse().unwrap(),
        count: 30,
        offset: 0,
        total:1
    };

    let json_output = serde_json::to_string(&param).expect("Erreur de sérialisation");

    let response = client
        .post(request_url)
        .header("Content-Type", "application/json")
        .header("X-Accept", "application/json")
        .body(json_output.to_owned())
        .send()
        .await?;

    let status = response.status();
    println!("{:?}", status);
    // let users: Vec<User> = response.json().await?;
    let users = response.text().await?;
    println!("{:?}", users);

    let json_value: Value = serde_json::from_str(&*users).expect("Erreur de parsing");

    let obj = json_value.as_object().unwrap();

    println!("maxActions: {}", obj["maxActions"].as_i64().unwrap_or(-1));
    println!("cachetype: {}", obj["cachetype"].as_str().unwrap_or("Inconnu"));
    
    // let vect = json_value.as_array().unwrap();
    // println!("{}", vect.len());
    // 
    // //println!("{}", vect.get(0)..len());
    // 
    // // Vérification et parcours du tableau JSON
    // if let Some(array) = json_value.as_array() {
    //     for utilisateur in array {
    //         let nom = utilisateur["login"].as_str().unwrap_or("Inconnu");
    //         let prenom = utilisateur["id"].as_i64().unwrap_or(-1);
    //         println!("Login: {}, Id: {}", nom, prenom);
    //     }
    // }

    Ok(())
}

fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config_file>", args[0]);
        std::process::exit(1);
    }
    let config_path = &args[1];

    // Lire le contenu du fichier
    let config_content = fs::read_to_string(config_path)?;

    // Parser le fichier TOML
    let config: Config = toml::from_str(&config_content)?;

    // Afficher la config chargée
    println!("Configuration chargée : {:?}", config);

    Ok(config)
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     println!("Hello, world!");
//
//     // unwrapped result from get_text() with ?
//     let text = get_text()?;
//
//     // changed formatting
//     println!("Got text {}", text);
//
//     // as main method response type is changes return unit type in case of success
//     Ok(())
// }
//
// fn get_text() -> Result<String, Box<dyn std::error::Error>> {
//     let http_client = reqwest::blocking::Client::new();
//     // let http_client = Client::new();
//     let url = "https://google.com";
//
//     let response = http_client
//         // form a get request with get(url)
//         .get(url)
//         // send the request and get Response or else return the error
//         .send()?
//         // get text from response or else return the error
//         .text().unwrap();
//
//     // wrapped response in Result
//     Ok(response)
// }
