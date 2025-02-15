use chrono::{DateTime, FixedOffset, Local};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::Handle;
use reqwest;
use reqwest::Error;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::cmp::{max, min};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, fmt, fs, thread};

#[derive(Debug, Deserialize, Clone)]
struct Config {
    url: String,
    consumer_key: String,
    access_token: String,
    repertoire: String,
    temporisation: u64,
    config_log: String,
    sauvegarde: u64,
    rechargement: ConfigRechargement
}

#[derive(Debug, Deserialize, Clone)]
struct ConfigRechargement {
    date_debut: String,
    dates: Vec<String>,
    nb_jours: i32,
    nb_parcourt: i32
}

#[derive(Serialize, Deserialize, Debug)]
struct Parameters {
    consumer_key: String,
    access_token: String,
    #[serde(rename = "detail_type")]
    detail_type: String,
    count: u64,
    offset: u64,
    total: u8,
    sort: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<u64>,
}

const DATA_ETAT: &str = "etat";
const DATA_OFFSET: &str = "offset";
const DATA_DATE: &str = "date";
const DATA_LISTE: &str = "liste";

const DATA_ETAT_INITIALISATION: &str = "initialisation";
const DATA_ETAT_MISE_A_JOUR: &str = "miseAJour";

impl fmt::Display for Parameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(
            f,
            "detail_type: {}, count: {}, offset: {}, total: {}, sort: {}, since: {:?}",
            self.detail_type, self.count, self.offset, self.total, self.sort, self.since
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let start = Local::now();
    println!("debut : {}", start.format("%Y-%m-%d %H:%M:%S"));

    let handle = init_logs();

    log::info!("debut : {}", start.format("%Y-%m-%d %H:%M:%S"));

    let config: Config = init_config(handle)?;

    let args: Vec<String> = env::args().collect();
    dbg!(&args);

    //let arg0: Vec<String>=args.iter().map(|x| x.clone()).collect();
    let arg0: Vec<String> = args.clone();
    if arg0.len() >= 1 {
        log::info!("param1 : {}", arg0[0]);
    }
    if arg0.len() >= 2 {
        log::info!("param2 : {}", arg0[1]);
    }
    let mut date_opt: Option<DateTime<FixedOffset>> = None;
    let mut nb_count_max = -1;
    let mut max_jours = 10;
    if arg0.len() >= 3 {
        log::info!("param3 : {}", arg0[2]);
        let s = arg0[2].clone();
        let s2 = s + "T00:00:00+00:00";
        let datetime = DateTime::parse_from_rfc3339(s2.as_str()).unwrap();
        date_opt = Some(datetime);
        nb_count_max = 2;
        if arg0.len() >= 4 {
            log::info!("param4 : {}", arg0[3]);
            let s = arg0[3].clone();
            let nb = s.parse::<i32>().unwrap_or(0);
            if nb > 0 {
                max_jours = nb;
            }
        }
    } else {
        let config2=config.clone();
        if !config2.rechargement.date_debut.trim().is_empty() {
            let s=config2.rechargement.date_debut.trim().to_string();
            log::info!("config date : {}", s);
            let s2 = s + "T00:00:00+00:00";
            let datetime = DateTime::parse_from_rfc3339(s2.as_str()).unwrap();
            date_opt = Some(datetime);
            nb_count_max = 2;            
            if config2.rechargement.nb_jours>0 {
                max_jours=config2.rechargement.nb_jours;
            }
            if config2.rechargement.nb_parcourt>0 {
                nb_count_max=config2.rechargement.nb_parcourt;
            }
        }
    }

    log::info!("date : {:?}, nb_count_max : {}, max_jours : {}", date_opt,nb_count_max, max_jours);

    let fichier = config.repertoire.clone() + "/data.json";

    let config2 = config.clone();
    backup_data(config2, &fichier.clone()).unwrap();

    match date_opt {
        Some(date) => {
            for i in 0..max_jours {
                let date2 = date + chrono::Duration::days(i as i64);
                log::info!("traitement de : {}", date2);
                traitement(config.clone(), Some(date2), nb_count_max, fichier.clone()).await;
            }
        }
        None => {
            traitement(config, date_opt, nb_count_max, fichier.clone()).await;
        }
    }

    let end = Local::now();

    let diff = end - start;

    log::info!("fin : {}", end.format("%Y-%m-%d %H:%M:%S"));

    log::info!("duree totale : {}", diff);
    Ok(())
}

fn init_logs() -> Handle {
    let stdout = ConsoleAppender::builder().build();

    let config = log4rs::config::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    let handle = log4rs::init_config(config).unwrap();
    handle
}

fn init_config(handle: Handle) -> Result<Config, Error> {
    let config_or_err = get_config(handle);

    let config: Config;
    match config_or_err {
        Ok(valeur) => {
            log::info!("Résultat : {:?}", valeur);
            config = valeur
        }
        Err(erreur) => {
            log::error!("Erreur : {}", erreur);
            std::process::exit(1);
        }
    }

    log::info!("Configuration chargée : {:?}", config);

    Ok(config)
}

async fn traitement(
    config: Config,
    date_opt: Option<DateTime<FixedOffset>>,
    nb_count_max: i32,
    fichier: String,
) {
    let nb_appel_max: u64;

    if nb_count_max > 0 {
        nb_appel_max = nb_count_max.try_into().unwrap();
    } else {
        //nb_appel_max = 3;
        //nb_appel_max = 10;
        nb_appel_max = 0;
    }

    log::info!("logging configure");

    let request_url2 = config.url.clone();
    log::info!("{}", request_url2);

    //let fichier = config.repertoire.clone() + "/data.json";

    // let config2 = config.clone();
    // backup_data(config2, &fichier.clone()).unwrap();

    let mut count = 0u64;
    let mut offset = 0u64;
    let mut since = 0u64;
    let mut dernier_since = 0u64;

    let mut data: Value;

    let initialisation: bool;

    let nb_sauvegarde: u64;

    if config.sauvegarde > 0 {
        nb_sauvegarde = config.sauvegarde;
    } else {
        nb_sauvegarde = 10;
    }

    let is_present = Path::new(&fichier.clone()).exists();
    if is_present {
        let file = File::open(fichier.clone()).expect("file should open read only");
        let json: Value = serde_json::from_reader(file).expect("file should be proper JSON");

        data = json;
        offset = data[DATA_OFFSET].as_u64().unwrap_or(0);
        initialisation = data[DATA_ETAT].as_str().unwrap_or("") == DATA_ETAT_INITIALISATION;
        if !initialisation {
            if date_opt.is_none() {
                since = data[DATA_DATE].as_u64().unwrap();
                log::info!("since file: {}", since);
            } else {
                since = date_opt.unwrap().timestamp().unsigned_abs();
                offset = 0;
                log::info!("since param: {}", since);
            }
        }
        if data.is_object() && data.as_object().unwrap().contains_key(DATA_LISTE) {
            let obj = data
                .as_object()
                .unwrap()
                .get(DATA_LISTE)
                .unwrap()
                .as_object()
                .unwrap();
            log::info!("taille au debut: {}", obj.len());
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

    let mut total_ajout = 0;
    let mut total_modifie = 0;

    let consumer_key = config.consumer_key.clone();
    let access_token = config.access_token.clone();

    let param: Parameters;

    if initialisation {
        param = Parameters {
            consumer_key: consumer_key,
            access_token: access_token,
            //detail_type: "complete".parse().unwrap(),
            detail_type: "simple".parse().unwrap(),
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
            //detail_type: "complete".parse().unwrap(),
            detail_type: "simple".parse().unwrap(),
            count: 30,
            offset: offset,
            total: 1,
            sort: "oldest".parse().unwrap(),
            since: Option::Some(since),
        };
    }

    log::info!("parametre de démarrage : {}", param);

    loop {
        let client = reqwest::Client::new();

        let json_output = serde_json::to_string(&param).expect("Erreur de sérialisation");

        let request_url = config.url.clone();

        log::info!("appel serveur offset: {}, since: {:?}", offset, param.since);

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
                    log::info!("OK");
                    log::info!("headers: {:?}", resp.headers());
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

        let json_value: Value = serde_json::from_str(&*body_ok).expect("Erreur de parsing");

        let obj = json_value.as_object().unwrap();

        if obj.contains_key("maxActions") {
            log::info!("maxActions: {}", obj["maxActions"].as_i64().unwrap_or(-1));
        }
        if obj.contains_key("cachetype") {
            log::info!(
                "cachetype: {}",
                obj["cachetype"].as_str().unwrap_or("Inconnu")
            );
        }
        if obj.contains_key("since") {
            log::info!("since: {}", obj["since"].as_i64().unwrap_or(-1));
        }

        let mut fin = false;
        if obj.contains_key("list") && obj["list"].is_object() {
            let obj2 = obj["list"].as_object().unwrap();

            log::info!("nb: {}", obj2.len());

            if obj2.len() == 0 {
                fin = true;
            } else {
                offset = offset + obj2.len() as u64;

                let liste = &mut data[DATA_LISTE];
                let mut nb_ajout = 0;
                let mut nb_remplace = 0;
                let mut s = "".to_string();
                let mut min_added = 0;
                let mut max_added = 0;
                let mut last_added = 0;
                let mut ordered_added = true;
                let mut min_updated = 0;
                let mut max_updated = 0;
                let mut last_updated = 0;
                let mut ordered_updated = true;
                for tmp in obj2.iter() {
                    if !liste.as_object().unwrap().contains_key(tmp.0) {
                        nb_ajout += 1;
                    } else {
                        nb_remplace += 1;
                    }
                    liste[tmp.0] = tmp.1.clone();
                    let res = tmp.1.clone();
                    let mut time_added = 0;
                    let mut time_updated = 0;
                    if res.is_object() {
                        let res2 = res.as_object().unwrap();
                        if res2.contains_key("time_added") {
                            time_added = res2["time_added"]
                                .as_str()
                                .unwrap_or("")
                                .parse::<i32>()
                                .unwrap_or(0);
                        }
                        if res2.contains_key("time_updated") {
                            time_updated = res2["time_updated"]
                                .as_str()
                                .unwrap_or("")
                                .parse::<i32>()
                                .unwrap_or(0);
                        }
                    }
                    if time_added > 0 {
                        if min_added == 0 {
                            min_added = time_added;
                        } else {
                            min_added = min(min_added, time_added);
                        }
                        if max_added == 0 {
                            max_added = time_added;
                        } else {
                            max_added = max(max_added, time_added);
                        }
                        if last_added > 0 {
                            if ordered_added {
                                if last_added > time_added {
                                    ordered_added = false;
                                }
                            }
                        }
                        last_added = time_added;
                    }
                    if time_updated > 0 {
                        if min_updated == 0 {
                            min_updated = time_updated;
                        } else {
                            min_updated = min(min_updated, time_updated);
                        }
                        if max_updated == 0 {
                            max_updated = time_updated;
                        } else {
                            max_updated = max(max_updated, time_updated);
                        }
                        if last_updated > 0 {
                            if ordered_updated {
                                if last_updated > time_updated {
                                    ordered_updated = false;
                                }
                            }
                        }
                        last_updated = time_updated;
                    }
                    let s0 = format!(
                        "({},{},{},{})",
                        tmp.0,
                        time_added,
                        time_updated,
                        time_added == time_updated
                    );
                    if s.len() > 0 {
                        s.push_str(",")
                    }
                    s.push_str(s0.as_str());
                }
                log::info!("nb_ajout: {}, nb_remplace: {}", nb_ajout, nb_remplace);
                log::info!("elements: {}", s);
                log::info!(
                    "added: ({},{},{}), updated: ({},{},{})",
                    min_added,
                    max_added,
                    ordered_added,
                    min_updated,
                    max_updated,
                    ordered_updated
                );
                total_ajout += nb_ajout;
                total_modifie += nb_remplace;
                data[DATA_OFFSET] = Value::Number(Number::from(offset));
                let date = obj["since"].as_i64().unwrap_or(-1);
                if date > 0 {
                    dernier_since = date as u64;
                    log::info!("dernier: {}", dernier_since);
                }
            }
        } else {
            fin = true;
        }
        if fin {
            log::info!("Pas de liste");
            if dernier_since > 0 {
                data[DATA_DATE] = Value::Number(Number::from(dernier_since));
                //data[DATA_OFFSET] = Value::Number(Number::from(0));
                log::info!("mise à jour du since: {}", dernier_since);
            }
            if initialisation && false {
                log::info!("fin d'initialisation");
                data[DATA_ETAT] = Value::String(DATA_ETAT_MISE_A_JOUR.to_string());
                log::info!("mise à jour de l'etat: {}", data[DATA_ETAT]);
            }
            break;
        }

        count += 1;

        log::info!("count : {}", count);
        log::info!(
            "total_ajout: {}, total_modifie: {}",
            total_ajout,
            total_modifie
        );

        let taille_totale: usize;
        match data[DATA_LISTE].as_object() {
            Some(obj) => {
                taille_totale = obj.len();
            }
            _ => taille_totale = 0,
        };
        log::info!("taille_totale: {}", taille_totale);

        if nb_appel_max > 0 && count >= nb_appel_max {
            log::info!("fin de boucle : {}", count);
            break;
        }

        if count % nb_sauvegarde == 0 {
            save_as_json_list(&data, &fichier);
        }

        if config.temporisation > 0 {
            thread::sleep(Duration::from_millis(config.temporisation));
        }
    }

    log::info!("nb total: {}", data.as_object().unwrap().len());

    log::info!("termine : {}", count);

    save_as_json_list(&data, &fichier);
}

fn backup_data(config: Config, fichier: &String) -> std::io::Result<()> {
    let date = Local::now();

    let is_present = Path::new(fichier).exists();
    if is_present {
        let mut s2: String = "".to_owned();
        let s = date.timestamp().to_string();
        let rep = config.repertoire.as_str();
        s2.push_str("/backup/data_");
        s2.push_str(&s);
        s2.push_str(".json");
        let file_resultat = format!("{rep}/backup/data_{s}.json");
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
    let chemin_config_log = config.config_log.as_str();
    let configuration_log =
        log4rs::config::load_config_file(chemin_config_log, Default::default())?;
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
