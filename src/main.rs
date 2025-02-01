use reqwest;
use reqwest::{Client, Response, StatusCode};

use reqwest::header::USER_AGENT;
use reqwest::Error;
use serde::{Deserialize, Serialize};

use serde_json::Value;

use std::fs::File;
use std::io::Write;
use std::{env, fs};
use std::path::Path;

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

    // let mut request_url = format!(
    //     "https://api.github.com/repos/{owner}/{repo}/stargazers",
    //     //config.url,
    //     owner = "rust-lang-nursery",
    //     repo = "rust-cookbook"
    // );
    let request_url2 = config.url.clone();
    println!("{}", request_url2);

    let fichier = config.repertoire + "/data.json";
    
    let mut count = 0u64;
    let mut offset = 0u64;

    let mut data:Value;

    let is_present = Path::new(&fichier.clone()).exists();
    if is_present {
        let file = fs::File::open(fichier.clone())
            .expect("file should open read only");
        let json: serde_json::Value = serde_json::from_reader(file)
            .expect("file should be proper JSON");

        //data= serde_json::json!({});
        data=json;
    } else {
        data= serde_json::json!({});
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
        
        let bodyOk:String;
        
        match response {
            Ok(resp) => match resp.status() {
                StatusCode::OK => {
                    match resp.text().await {
                        Ok(body) => {
                            // println!("Réponse reçue : {}", body)
                            bodyOk=body;
                        },
                        Err(err) => {
                            eprintln!("Erreur en lisant la réponse : {}", err);
                            break;
                        },
                    }
                }
                StatusCode::NOT_FOUND => {
                    eprintln!("Erreur 404 : Ressource non trouvée.");
                    break;
                }
                StatusCode::BAD_REQUEST => {
                    eprintln!("Erreur 400 : Bad request.");
                    break;
                }
                other => {
                    eprintln!("Réponse inattendue : {:?}", other);
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
        let users=bodyOk;
        println!("{:?}", users);

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

            for tmp in obj2.iter() {
                data[tmp.0] = tmp.1.clone();
            }
        } else {
            println!("Pas de liste");
            break;
        }

        count += 1;

        println!("count : {}", count);

        if count > 3 {
            println!("fin de boucle : {}", count);
            break;
        }
    }

    println!("nb total: {}", data.as_object().unwrap().len());

    println!("termine : {}", count);

    

    save_as_json_list(&data, &fichier);

    println!("Fichier sauve: {}", fichier);
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

fn save_as_json_list(list: &Value, fname: &str) {
    let list_as_json = serde_json::to_string(list).unwrap();

    let mut file = File::create(fname).expect("Could not create file!");

    file.write_all(list_as_json.as_bytes())
        .expect("Cannot write to the file!");
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
