use reqwest;
use reqwest::{Client, Response, StatusCode};

use reqwest::header::USER_AGENT;
use reqwest::Error;
use serde::{Deserialize, Serialize};

use serde_json::{Number, Value};

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, fs, thread};

use chrono::Local;

#[derive(Debug, Deserialize)]
struct Config {
    //app_name: String,
    //debug: bool,
    //port: u16,
    url: String,
    consumer_key: String,
    access_token: String,
    repertoire: String,
    temporisation: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Parameters {
    consumer_key: String,
    access_token: String,
    detailType: String,
    count: u64,
    offset: u64,
    total: u8,
    sort: String,
}

#[derive(Deserialize, Debug)]
struct User {
    login: String,
    id: u32,
}

const DATA_ETAT:&str = "etat";
const DATA_OFFSET:&str = "offset";
const DATA_DATE:&str = "date";
const DATA_LISTE:&str = "liste";

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

    let nb_appel_max: u64;

    let start = Local::now();
    println!("debut : {}", start.format("%Y-%m-%d %H:%M:S"));


    //nb_appel_max = 10;
    nb_appel_max = 0;

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

    let request_url2 = config.url.clone();
    println!("{}", request_url2);

    let fichier = config.repertoire + "/data.json";

    let mut count = 0u64;
    let mut offset = 0u64;

    let mut data: Value;

    let is_present = Path::new(&fichier.clone()).exists();
    if is_present {
        let file = fs::File::open(fichier.clone()).expect("file should open read only");
        let json: serde_json::Value =
            serde_json::from_reader(file).expect("file should be proper JSON");

        //data= serde_json::json!({});
        data = json;
        offset = data[DATA_OFFSET].as_u64().unwrap_or(0)
    } else {
        data = serde_json::json!({
            DATA_ETAT:"initialisation",
            DATA_OFFSET: 0,
            DATA_DATE:0,
            DATA_LISTE: {}
        });
    }

    loop {
        let client = reqwest::Client::new();
        // let response = client
        //     .get(request_url)
        //     .header(USER_AGENT, "rust-web-api-client") // gh api requires a user-agent header
        //     .send()
        //     .await?;

        //let json_data = r#"{"title":"Problems during installation","status":"todo","priority":"medium","label":"bug"}"#;
        //let json_data = "{\"title":"Problems during installation","status":"todo","priority":"medium","label":"bug"}"#;

        let consumer_key = config.consumer_key.clone();
        let access_token = config.access_token.clone();

        let param = Parameters {
            consumer_key: consumer_key,
            access_token: access_token,
            //detailType: "complete".parse().unwrap(),
            detailType: "simple".parse().unwrap(),
            count: 30,
            offset: offset,
            total: 1,
            sort: "oldest".parse().unwrap(),
        };

        let json_output = serde_json::to_string(&param).expect("Erreur de sérialisation");

        let request_url = config.url.clone();

        println!("appel serveur offset : {}", offset);

        let response = client
            .post(request_url)
            .header("Content-Type", "application/json")
            .header("X-Accept", "application/json")
            .body(json_output.to_owned())
            .send()
            .await;

        let bodyOk: String;

        match response {
            Ok(resp) => match resp.status() {
                StatusCode::OK => {
                    match resp.text().await {
                        Ok(body) => {
                            // println!("Réponse reçue : {}", body)
                            bodyOk = body;
                        }
                        Err(err) => {
                            eprintln!("Erreur en lisant la réponse : {}", err);
                            break;
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    eprintln!("Erreur 404 : Ressource non trouvée.");
                    eprintln!("headers: {:?}", resp.headers());
                    eprintln!("body: {:?}", resp.text().await);
                    break;
                }
                StatusCode::BAD_REQUEST => {
                    eprintln!("Erreur 400 : Bad request.");
                    eprintln!("headers: {:?}", resp.headers());
                    eprintln!("body: {:?}", resp.text().await);
                    break;
                }
                other => {
                    eprintln!("Réponse inattendue : {:?}", other);
                    eprintln!("headers: {:?}", resp.headers());
                    eprintln!("body: {:?}", resp.text().await);
                    break;
                }
            },
            Err(err) => {
                eprintln!("Erreur lors de la requête : {}", err);
                break;
            }
        }

        // let status = response.status();
        // println!("{:?}", status);
        //
        // match (status) {
        //     StatusCode(StatusCode.Ok) => {}
        //     StatusCode(_) => {}
        // }

        // let users: Vec<User> = response.json().await?;
        //let users = response.text().await?;
        let users = bodyOk;
        //println!("{:?}", users);

        let json_value: Value = serde_json::from_str(&*users).expect("Erreur de parsing");

        let obj = json_value.as_object().unwrap();

        println!("maxActions: {}", obj["maxActions"].as_i64().unwrap_or(-1));
        println!(
            "cachetype: {}",
            obj["cachetype"].as_str().unwrap_or("Inconnu")
        );
        println!("since: {}", obj["since"].as_i64().unwrap_or(-1));
        //println!("total: {}", obj["total"].as_i64().unwrap_or(-1));

        // let vec = obj["list"].as_array().unwrap();
        // println!("vect nb: {}", vec.len());

        if obj["list"].is_object() {
            let obj2 = obj["list"].as_object().unwrap();

            println!("nb: {}", obj2.len());
            offset = offset + obj2.len() as u64;

            let liste = &mut data[DATA_LISTE];
            for tmp in obj2.iter() {
                liste[tmp.0] = tmp.1.clone();
            }
            data[DATA_OFFSET] = Value::Number(Number::from(offset));
        } else {
            println!("Pas de liste");
            break;
        }

        count += 1;

        println!("count : {}", count);

        if nb_appel_max > 0 && count >= nb_appel_max {
            println!("fin de boucle : {}", count);
            break;
        }

        if count % 10 == 0 {
            save_as_json_list(&data, &fichier);
            println!("Fichier sauve: {}", fichier);
        }

        if config.temporisation > 0 {
            thread::sleep(Duration::from_millis(config.temporisation));
        }
    }

    println!("nb total: {}", data.as_object().unwrap().len());

    println!("termine : {}", count);

    save_as_json_list(&data, &fichier);

    println!("Fichier sauve: {}", fichier);

    let end = Local::now();

    let diff = end - start;

    println!("fin : {}", end.format("%Y-%m-%d %H:%M:S"));

    println!("duree totale : {}", diff);
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

fn save_as_json_list(list: &Value, fname: &str) {
    let list_as_json = serde_json::to_string(list).unwrap();

    let mut file = File::create(fname).expect("Could not create file!");

    file.write_all(list_as_json.as_bytes())
        .expect("Cannot write to the file!");
}

