mod minmax;

use chrono::{DateTime, FixedOffset, Local, NaiveTime};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::Handle;
use reqwest;
use reqwest::Error;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, fmt, fs, thread};
use crate::minmax::create_min_max;

#[derive(Debug, Deserialize, Clone)]
struct Config {
    url: String,
    consumer_key: String,
    access_token: String,
    repertoire: String,
    temporisation: u64,
    config_log: String,
    sauvegarde: u64,
    rechargement: ConfigRechargement,
}

#[derive(Debug, Deserialize, Clone)]
struct ConfigRechargement {
    date_debut: String,
    dates: Vec<String>,
    nb_jours: i32,
    nb_parcourt: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigParam {
    pub date_dernier_traiment: u64,
    pub offset: i64,
    pub etat: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ConfigParamForce {
    pub date_opt: Option<DateTime<FixedOffset>>,
    pub nb_count_max: i32,
    pub force: bool,
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

#[derive(Debug)]
enum ListeDates {
    DatesContinues(DateTime<FixedOffset>, u32, i32),
    ListeDates(Vec<DateTime<FixedOffset>>, i32),
    DateJusquaFin(i32),
    None,
}

const DATA_ETAT: &str = "etat";
const DATA_OFFSET: &str = "offset";
const DATA_DATE: &str = "date";
const DATA_LISTE: &str = "liste";

const DATA_ETAT_INITIALISATION: &str = "initialisation";
const DATA_ETAT_MISE_A_JOUR: &str = "miseAJour";
const DATA_ETAT_SPECIFIQUE: &str = "specifique";

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
    let mut dates: ListeDates = ListeDates::None;
    if arg0.len() >= 3 {
        log::info!("param3 : {}", arg0[2]);
        let s = arg0[2].clone();
        let nb_count_max = 2;
        let mut max_jours = 10;
        let date=parse_date(s);
        if arg0.len() >= 4 {
            log::info!("param4 : {}", arg0[3]);
            let s = arg0[3].clone();
            let nb = s.parse::<i32>().unwrap_or(0);
            if nb > 0 {
                max_jours = nb;
            }
        }
        dates = ListeDates::DatesContinues(date, max_jours as u32, nb_count_max);
    } else {
        let config2 = config.clone();
        if !config2.rechargement.date_debut.trim().is_empty() {
            let s = config2.rechargement.date_debut.trim().to_string();
            let mut nb_count_max = 2;
            let mut max_jours = 10;
            log::info!("config date : {}", s);
            let date=parse_date(s);
            if config2.rechargement.nb_jours > 0 {
                max_jours = config2.rechargement.nb_jours;
            }
            if config2.rechargement.nb_parcourt > 0 {
                nb_count_max = config2.rechargement.nb_parcourt;
            }
            dates = ListeDates::DatesContinues(date, max_jours as u32, nb_count_max);
        } else if !config2.rechargement.dates.is_empty() {
            log::info!("config dates : {:?}", config2.rechargement.dates);
            let mut nb_count_max = 2;
            let mut liste_dates: Vec<DateTime<FixedOffset>> = Vec::new();
            for date_str in &config2.rechargement.dates {
                let d = date_str.clone();
                let s2 = d + "T00:00:00+00:00";
                let datetime = DateTime::parse_from_rfc3339(s2.as_str()).unwrap();
                liste_dates.push(datetime);
            }
            if config2.rechargement.nb_parcourt > 0 {
                nb_count_max = config2.rechargement.nb_parcourt;
            }
            dates = ListeDates::ListeDates(liste_dates, nb_count_max);
        }
    }

    log::info!("date : {:?}", dates);

    let fichier = config.repertoire.clone() + "/data.json";
    let fichier_param = config.repertoire.clone() + "/param.json";

    let config2 = config.clone();
    backup_data(config2, &fichier.clone(), &"data".to_string()).unwrap();
    let config3 = config.clone();
    backup_data(
        config3.clone(),
        &fichier_param.clone(),
        &"param".to_string(),
    )
    .unwrap();

    let config_param = init_config_param(fichier_param.clone());

    match dates {
        ListeDates::DatesContinues(date, max_jours, nb_count_max) => {
            log::info!("parcourt de dates consecutives");
            for i in 0..max_jours {
                let date2 = date + chrono::Duration::days(i as i64);
                log::info!("traitement de : {}", date2);
                traitement_specifique(config.clone(),nb_count_max,
                                      fichier.clone(), fichier_param.clone(),
                                      config_param.clone(),&date2).await;
            }
        }
        ListeDates::ListeDates(liste_dates, nb_count_max) => {
            log::info!("parcourt de dates");
            for date in liste_dates.iter() {
                log::info!("traitement de : {}", date);
                traitement_specifique(config.clone(),nb_count_max,fichier.clone(), 
                                      fichier_param.clone(),config_param.clone(),date).await;
            }
        }
        ListeDates::DateJusquaFin(nb_count_max) => {
            log::info!("mise à jours");
            let config_force = ConfigParamForce {
                date_opt: None,
                nb_count_max,
                force: false,
            };
            traitement(
                config,
                config_force,
                fichier.clone(),
                fichier_param.clone(),
                config_param.clone(),
            )
            .await;
        }
        ListeDates::None => {
            log::info!("aucun traitement à réaliser");
        }
    }

    let end = Local::now();

    let diff = end - start;

    log::info!("fin : {}", end.format("%Y-%m-%d %H:%M:%S"));

    log::info!("duree totale : {}", diff);
    Ok(())
}

async fn traitement_specifique(config:Config, nb_count_max:i32, fichier: String, fichier_param:String, mut config_param:ConfigParam, date: &DateTime<FixedOffset>){
    config_param.etat = DATA_ETAT_SPECIFIQUE.to_string();
    config_param.offset = 0;
    config_param.date_dernier_traiment = date.timestamp() as u64;
    let config_force = ConfigParamForce {
        date_opt: Some(*date),
        nb_count_max: nb_count_max,
        force: true,
    };
    traitement(
        config.clone(),
        config_force,
        fichier.clone(),
        fichier_param.clone(),
        config_param.clone(),
    )
        .await;
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

fn parse_date(s:String) -> DateTime<FixedOffset> {
    let test = s.parse::<u64>();
    match test {
        Ok(ok) => {
            let datetime = DateTime::from_timestamp_millis(ok as i64 * 1000)
                .unwrap()
                .fixed_offset();
            datetime
        }
        Err(_e) => {
            let s2 = s + "T00:00:00+00:00";
            let datetime = DateTime::parse_from_rfc3339(s2.as_str()).unwrap();
            datetime
        }
    }
}

async fn traitement(
    config: Config,
    config_force: ConfigParamForce,
    fichier: String,
    fichier_param: String,
    mut data_param: ConfigParam,
) {
    let nb_appel_max: u64;

    if config_force.force && config_force.nb_count_max > 0 {
        nb_appel_max = config_force.nb_count_max.try_into().unwrap();
    } else {
        //nb_appel_max = 3;
        //nb_appel_max = 10;
        nb_appel_max = 0;
    }

    log::info!("logging configure");

    let request_url2 = config.url.clone();
    log::info!("{}", request_url2);

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
        // offset = data[DATA_OFFSET].as_u64().unwrap_or(0);
        offset = data_param.offset as u64;
        //initialisation = data[DATA_ETAT].as_str().unwrap_or("") == DATA_ETAT_INITIALISATION;
        initialisation = data_param.etat.as_str() == DATA_ETAT_INITIALISATION;
        if !initialisation {
            if !config_force.force {
                if data_param.date_dernier_traiment > 0 {
                    since = data_param.date_dernier_traiment;
                }
                log::info!("since file: {}", since);
            } else {
                since = config_force.date_opt.unwrap().timestamp().unsigned_abs();
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
            consumer_key,
            access_token,
            //detail_type: "complete".parse().unwrap(),
            detail_type: "simple".parse().unwrap(),
            count: 30,
            offset,
            total: 1,
            sort: "oldest".parse().unwrap(),
            since: Option::None,
        };
    } else {
        check_timestamp(&since);
        param = Parameters {
            consumer_key,
            access_token,
            //detail_type: "complete".parse().unwrap(),
            detail_type: "simple".parse().unwrap(),
            count: 30,
            offset,
            total: 1,
            sort: "oldest".parse().unwrap(),
            since: Option::Some(since),
        };
    }

    log::info!("parametre de démarrage : {}", param);
    log::info!("parametre config force : {:?}", config_force);
    log::info!("parametre data_param : {:?}", data_param);

    loop {
        let client = reqwest::Client::new();

        let json_output = serde_json::to_string(&param).expect("Erreur de sérialisation");

        let request_url = config.url.clone();

        log::info!(
            "appel serveur offset: {}, since: {:?} ({:?})",
            offset,
            param.since,
            DateTime::from_timestamp(param.since.unwrap_or(0) as i64, 0).unwrap()
        );

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
                let mut ajout = create_min_max();
                let mut remplace = create_min_max();
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
                        ajout.add(time_added);
                    }
                    if time_updated > 0 {
                        remplace.add(time_updated);
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
                log::debug!("elements: {}", s);
                log::info!("added: {}, updated: {}", ajout, remplace);
                total_ajout += nb_ajout;
                total_modifie += nb_remplace;
                if data_param.etat == DATA_ETAT_SPECIFIQUE {
                    data_param.offset = offset as i64;
                }
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
                //data[DATA_DATE] = Value::Number(Number::from(dernier_since));
                //data[DATA_OFFSET] = Value::Number(Number::from(0));
                // let d = UNIX_EPOCH + Duration::from_secs(dernier_since);
                // // Create DateTime from SystemTime
                // let datetime = DateTime::<Utc>::from(d);
                // // Formats the combined date and time with the specified format string.
                // let timestamp_str = datetime.format("%Y-%m-%d").to_string();
                check_timestamp(&dernier_since);

                //if data_param.etat==DATA_ETAT_MISE_A_JOUR.to_string() {
                if !config_force.force {
                    data_param.offset = 0;
                    //let debut_journee=true;
                    let debut_journee = false;
                    if debut_journee {
                        let naive = DateTime::from_timestamp(dernier_since as i64, 0).unwrap();
                        let date = naive
                            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                            .unwrap();
                        data_param.date_dernier_traiment = date.timestamp_millis() as u64;
                    } else {
                        //data_param.date_dernier_traiment = dernier_since;
                        let timestamp = (Local::now().timestamp_millis() / 1000) as u64;
                        check_timestamp(&timestamp);
                        data_param.date_dernier_traiment = timestamp;
                    }
                    log::info!(
                        "mise à jour du since: {} ({:?})",
                        dernier_since,
                        DateTime::from_timestamp(dernier_since as i64, 0).unwrap()
                    );
                    log::info!("mise à jour offset: {}", data_param.offset);
                } else {
                    data_param.date_dernier_traiment = dernier_since;
                    log::info!(
                        "mise à jour du since: {} ({:?})",
                        dernier_since,
                        DateTime::from_timestamp(dernier_since as i64, 0).unwrap()
                    );
                }
            }
            if initialisation && false {
                log::info!("fin d'initialisation");
                //data[DATA_ETAT] = Value::String(DATA_ETAT_MISE_A_JOUR.to_string());
                data_param.etat = DATA_ETAT_MISE_A_JOUR.to_string();
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
            save_as_json_list(&data, &fichier, &data_param, &fichier_param);
        }

        if config.temporisation > 0 {
            log::info!("temporisation : {} ms", config.temporisation);
            thread::sleep(Duration::from_millis(config.temporisation));
            log::info!("temporisation : {} ms OK", config.temporisation);
        }
    }

    log::info!("nb total: {}", data.as_object().unwrap().len());

    log::info!("termine : {}", count);

    save_as_json_list(&data, &fichier, &data_param, &fichier_param);
}

fn check_timestamp(since: &u64) {
    if *since > 1764601526 {
        panic!("{}", format!("since invalid: {since}"));
    }
    if *since <= 100 {
        panic!("{}", format!("since invalid: {since}"));
    }
}

fn init_config_param(fichier_param: String) -> ConfigParam {
    let data_param: ConfigParam;
    //let fichier_param = fichier_param; //fichier.clone()+"/../param.json";
    let is_present = Path::new(&fichier_param.clone()).exists();
    if is_present {
        let file = File::open(fichier_param.clone()).expect("file should open read only");
        data_param = serde_json::from_reader(file).expect("file should be proper JSON");
    } else {
        let p = ConfigParam {
            date_dernier_traiment: 0,
            offset: 0,
            etat: "".to_string(),
        };
        data_param = p;
    }
    data_param
}

fn backup_data(
    config: Config,
    fichier: &String,
    debut_nom_fichier: &String,
) -> std::io::Result<()> {
    let date = Local::now();

    let is_present = Path::new(fichier).exists();
    if is_present {
        let mut s2: String = "".to_owned();
        let s = date.timestamp().to_string();
        let rep = config.repertoire.as_str();
        s2.push_str(format!("/backup/{debut_nom_fichier}_").as_str());
        s2.push_str(&s);
        s2.push_str(".json");
        let file_resultat = format!("{rep}/backup/{debut_nom_fichier}_{s}.json");
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

fn save_as_json_list(list: &Value, fname: &str, param: &ConfigParam, fname_param: &String) {
    log::info!("Sauvegarde de {} ...", fname);
    let list_as_json = serde_json::to_string(list).unwrap();

    let mut file = File::create(fname).expect("Could not create file!");

    file.write_all(list_as_json.as_bytes())
        .expect("Cannot write to the file!");
    log::info!("Fichier {} sauve", fname);

    log::info!("Sauvegarde de {} ...", fname_param);

    let list_as_json = serde_json::to_string(param).unwrap();

    let mut file = File::create(fname_param).expect("Could not create file!");

    file.write_all(list_as_json.as_bytes())
        .expect("Cannot write to the file!");
    log::info!("Fichier {} sauve", fname_param);
}
