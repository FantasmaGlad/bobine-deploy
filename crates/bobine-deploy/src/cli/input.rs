//! Identifiants : jamais en argument (visibles dans la liste des processus), mais par variable d'environnement
//! (`BOBINE_SSH_PASSWORD`, `BOBINE_ROOT_PASSWORD`) ou une ligne de l'entrée standard.

use std::io::BufRead;

use bobine_deploy_proto::{Credentials, Error};

use super::args::Target;

pub fn credentials(target: &Target, input: &mut impl BufRead) -> Result<Credentials, Error> {
    let username = target
        .user
        .clone()
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| Error::usage("Utilisateur SSH inconnu : précisez --user."))?;
    let password = if target.password_stdin {
        Some(read_line(input, "mot de passe SSH")?)
    } else {
        std::env::var("BOBINE_SSH_PASSWORD").ok().filter(|p| !p.is_empty())
    };
    Ok(Credentials {
        host: target.host.clone(),
        port: target.port,
        username,
        password,
        key_path: target.key.clone(),
        local_keys: target.local_keys,
    })
}

pub fn read_line(input: &mut impl BufRead, what: &str) -> Result<String, Error> {
    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|e| Error::failed(format!("Lecture du {what} sur l'entrée standard impossible : {e}")))?;
    let value = line.trim_end_matches(['\r', '\n']).to_string();
    zeroize::Zeroize::zeroize(&mut line);
    if value.is_empty() {
        return Err(Error::usage(format!("Aucun {what} sur l'entrée standard.")));
    }
    Ok(value)
}
