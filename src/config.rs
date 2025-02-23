use std::{env, fs};
use std::fs::File;
use std::path::Path;
use chrono::{DateTime, FixedOffset};
use log4rs::Handle;
use serde::{Deserialize, Serialize};
use log4rs::{Config};

#[derive(Debug, Deserialize, Clone)]
pub struct Config2 {
    pub(crate) url: String,
    pub(crate) consumer_key: String,
    pub(crate) access_token: String,
    pub(crate) repertoire: String,
    pub(crate) temporisation: u64,
    config_log: String,
    pub(crate) sauvegarde: u64,
    pub(crate) rechargement: crate::config::ConfigRechargement,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigRechargement {
    pub(crate) date_debut: String,
    pub(crate) dates: Vec<String>,
    pub(crate) nb_jours: i32,
    pub(crate) nb_parcourt: i32,
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


pub const DATA_ETAT_INITIALISATION: &str = "initialisation";
pub const DATA_ETAT_MISE_A_JOUR: &str = "miseAJour";
pub const DATA_ETAT_SPECIFIQUE: &str = "specifique";


pub fn init_config_param(fichier_param: String) -> ConfigParam {
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


pub fn get_config(handle: Handle) -> Result<Config2, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config_file>", args[0]);
        std::process::exit(1);
    }
    let config_path = &args[1];

    // Lire le contenu du fichier
    let config_content = fs::read_to_string(config_path)?;

    // Parser le fichier TOML
    let config: Config2 = toml::from_str(&config_content)?;

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