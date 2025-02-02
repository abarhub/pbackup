//use log4rs::config::load_config_file;
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

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Logger, Root};
use log4rs::Handle;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    url: String,
    consumer_key: String,
    access_token: String,
    repertoire: String,
    temporisation: u64,
    config_log: String
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
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<u64>
}


const DATA_ETAT: &str = "etat";
const DATA_OFFSET: &str = "offset";
const DATA_DATE: &str = "date";
const DATA_LISTE: &str = "liste";

const DATA_ETAT_INITIALISATION: &str = "initialisation";
const DATA_ETAT_MISE_A_JOUR: &str = "miseAJour";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let nb_appel_max: u64;

    let start = Local::now();
    println!("debut : {}", start.format("%Y-%m-%d %H:%M:%S"));

    let stdout = ConsoleAppender::builder().build();
    
    let config = log4rs::config::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        //.appender(Appender::builder().build("requests", Box::new(requests)))
        //.logger(Logger::builder().build("app::backend::db", LevelFilter::Info))
        // .logger(Logger::builder()
        //     .appender("requests")
        //     .additive(false)
        //     .build("app::requests", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    
    

    let handle = log4rs::init_config(config).unwrap();
    
    //nb_appel_max = 3;
    //nb_appel_max = 10;
    nb_appel_max = 0;

    let config_or_err = get_config(handle);

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
    //println!("Configuration chargée : {:?}", config);
    log::info!("Configuration chargée : {:?}", config);

    log::info!("logging configure");

    let request_url2 = config.url.clone();
    log::info!("{}", request_url2);

    let fichier = config.repertoire.clone() + "/data.json";

    let config2=config.clone();
    backup_data(config2, &fichier.clone());

    let mut count = 0u64;
    let mut offset = 0u64;
    let mut since =0u64;
    let mut dernier_since =0u64;

    let mut data: Value;

    let initialisation: bool;

    let is_present = Path::new(&fichier.clone()).exists();
    if is_present {
        let file = fs::File::open(fichier.clone()).expect("file should open read only");
        let json: serde_json::Value =
            serde_json::from_reader(file).expect("file should be proper JSON");

        //data= serde_json::json!({});
        data = json;
        offset = data[DATA_OFFSET].as_u64().unwrap_or(0);
        initialisation = data[DATA_ETAT].as_str().unwrap_or("") == DATA_ETAT_INITIALISATION;
        if !initialisation {
            since=data[DATA_DATE].as_u64().unwrap();
        }
    } else {
        data = serde_json::json!({
            DATA_ETAT:DATA_ETAT_INITIALISATION,
            DATA_OFFSET: 0,
            DATA_DATE:0,
            DATA_LISTE: {}
        });
        initialisation = true;
    }

    loop {
        let client = reqwest::Client::new();

        let consumer_key = config.consumer_key.clone();
        let access_token = config.access_token.clone();

        let param: Parameters;
        if initialisation {
            param = Parameters {
                consumer_key: consumer_key,
                access_token: access_token,
                //detailType: "complete".parse().unwrap(),
                detailType: "simple".parse().unwrap(),
                count: 30,
                offset: offset,
                total: 1,
                sort: "oldest".parse().unwrap(),
                since: Option::None,
            };
        } else {
            param = Parameters {
                consumer_key: consumer_key,
                access_token: access_token,
                //detailType: "complete".parse().unwrap(),
                detailType: "simple".parse().unwrap(),
                count: 30,
                offset: offset,
                total: 1,
                sort: "oldest".parse().unwrap(),
                since: Option::Some(since),
            };
        }

        let json_output = serde_json::to_string(&param).expect("Erreur de sérialisation");

        let request_url = config.url.clone();

        log::info!("appel serveur offset : {}", offset);

        let response = client
            .post(request_url)
            .header("Content-Type", "application/json")
            .header("X-Accept", "application/json")
            .body(json_output.to_owned())
            .send()
            .await;

        let body_ok: String;

        match response {
            Ok(resp) => match resp.status() {
                StatusCode::OK => {
                    match resp.text().await {
                        Ok(body) => {
                            // println!("Réponse reçue : {}", body)
                            body_ok = body;
                        }
                        Err(err) => {
                            log::error!("Erreur en lisant la réponse : {}", err);
                            break;
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    log::error!("Erreur 404 : Ressource non trouvée.");
                    log::error!("headers: {:?}", resp.headers());
                    log::error!("body: {:?}", resp.text().await);
                    break;
                }
                StatusCode::BAD_REQUEST => {
                    log::error!("Erreur 400 : Bad request.");
                    log::error!("headers: {:?}", resp.headers());
                    log::error!("body: {:?}", resp.text().await);
                    break;
                }
                other => {
                    log::error!("Réponse inattendue : {:?}", other);
                    log::error!("headers: {:?}", resp.headers());
                    log::error!("body: {:?}", resp.text().await);
                    break;
                }
            },
            Err(err) => {
                log::error!("Erreur lors de la requête : {}", err);
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


        let json_value: Value = serde_json::from_str(&*body_ok).expect("Erreur de parsing");

        let obj = json_value.as_object().unwrap();

        log::info!("maxActions: {}", obj["maxActions"].as_i64().unwrap_or(-1));
        log::info!(
            "cachetype: {}",
            obj["cachetype"].as_str().unwrap_or("Inconnu")
        );
        log::info!("since: {}", obj["since"].as_i64().unwrap_or(-1));
        //println!("total: {}", obj["total"].as_i64().unwrap_or(-1));

        // let vec = obj["list"].as_array().unwrap();
        // println!("vect nb: {}", vec.len());

        if obj["list"].is_object() {
            let obj2 = obj["list"].as_object().unwrap();

            log::info!("nb: {}", obj2.len());
            offset = offset + obj2.len() as u64;

            let liste = &mut data[DATA_LISTE];
            for tmp in obj2.iter() {
                liste[tmp.0] = tmp.1.clone();
            }
            data[DATA_OFFSET] = Value::Number(Number::from(offset));
            let date=obj["since"].as_i64().unwrap_or(-1);
            if date>0 {
                dernier_since= date as u64;
                log::info!("dernier: {}", dernier_since);
            }
        } else {
            log::info!("Pas de liste");
            if dernier_since>0{
                data[DATA_DATE] = Value::Number(Number::from(dernier_since));
                log::info!("mise à jour du since: {}", dernier_since);
            }
            if initialisation{
                log::info!("fin d'initialisation");
                data[DATA_ETAT] = Value::String(DATA_ETAT_MISE_A_JOUR.to_string());
            }
            break;
        }

        count += 1;

        log::info!("count : {}", count);

        if nb_appel_max > 0 && count >= nb_appel_max {
            log::info!("fin de boucle : {}", count);
            break;
        }

        if count % 10 == 0 {
            save_as_json_list(&data, &fichier);
        }

        if config.temporisation > 0 {
            thread::sleep(Duration::from_millis(config.temporisation));
        }
    }

    log::info!("nb total: {}", data.as_object().unwrap().len());

    log::info!("termine : {}", count);

    save_as_json_list(&data, &fichier);

    let end = Local::now();

    let diff = end - start;

    log::info!("fin : {}", end.format("%Y-%m-%d %H:%M:%S"));

    log::info!("duree totale : {}", diff);
    Ok(())
}

fn backup_data(config: Config, fichier: &String)-> std::io::Result<()> {
    let date = Local::now();

    // let is_present = Path::new(&fichier.clone()).exists();
    let is_present = Path::new(fichier).exists();
    if(is_present){
        let mut s2: String = "".to_owned();
        let s=date.timestamp().to_string();
        let rep=config.repertoire.as_str();
        // s2.push_str(&config.repertoire.as_str());
        s2.push_str("/backup/data_");
        s2.push_str(&s);
        s2.push_str(".json");
        let file_resultat=format!("{rep}/backup/data_{s}.json");
        fs::copy(fichier, &file_resultat)?;
        log::info!("copie vers : {}", file_resultat);
    }
    Ok(())
}

fn get_config(handle: Handle) -> Result<Config, Box<dyn std::error::Error>> {
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
    log::info!("Configuration chargée : {:?}", config);

    log::info!("Reconfiguration des logs ...");
    let chemin_config_log=config.config_log.as_str();
    let configuration_log=log4rs::config::load_config_file(chemin_config_log, Default::default())?;
    handle.set_config(configuration_log);
    log::info!("Reconfiguration des logs ok");
    
    Ok(config)
}

fn save_as_json_list(list: &Value, fname: &str) {
    log::info!("Sauvegarde de {} ...", fname);
    let list_as_json = serde_json::to_string(list).unwrap();

    let mut file = File::create(fname).expect("Could not create file!");

    file.write_all(list_as_json.as_bytes())
        .expect("Cannot write to the file!");
    log::info!("Fichier {} sauve", fname);
}

