use crate::cli::ConfigAction;
use crate::config::{
    get_global_setting, global_config_path, load_global_config, save_global_config,
    set_global_setting, unset_global_setting,
};

pub(crate) fn cmd_config(action: ConfigAction) -> Result<(), String> {
    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

    match action {
        ConfigAction::Get { key } => {
            println!("{}", get_global_setting(&global, &key)?);
        }
        ConfigAction::Set { key, value } => {
            let msg = set_global_setting(&mut global, &key, &value)?;
            save_global_config(&global_path, &global)?;
            println!("{msg}");
        }
        ConfigAction::Unset { key } => {
            let msg = unset_global_setting(&mut global, &key)?;
            save_global_config(&global_path, &global)?;
            println!("{msg}");
        }
    }

    Ok(())
}
