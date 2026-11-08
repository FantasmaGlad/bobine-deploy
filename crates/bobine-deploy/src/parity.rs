//! Parité des façades : chaque commande du catalogue existe dans la ligne de commande et l'interface
//! d'automatisation (et dans l'interface graphique, `gui::COMMANDS`).

#[cfg(test)]
mod tests {
    use bobine_deploy_proto::CATALOG;

    fn catalog() -> Vec<String> {
        CATALOG.iter().map(|c| c.name.to_string()).collect()
    }

    #[test]
    fn the_command_line_exposes_exactly_the_catalog() {
        assert_eq!(crate::cli::subcommand_names(), catalog());
    }

    #[test]
    fn the_automation_interface_serves_every_command() {
        let methods = crate::rpc::methods();
        for name in catalog() {
            assert!(methods.contains(&name), "méthode {name} absente");
        }
        assert_eq!(methods.len(), catalog().len() + crate::rpc::META_METHODS.len());
    }
}
