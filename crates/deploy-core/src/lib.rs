//! Logique pure de Bobine Deploy, sans aucune entrée ni sortie : contrat `install.sh --progress=json` ([`progress`]),
//! commande d'installation distante ([`orchestrate`]), choix de l'élévation ([`privilege`]) et arithmétique de la
//! découverte réseau ([`discovery`]).

pub mod discovery;
pub mod orchestrate;
pub mod privilege;
pub mod progress;
