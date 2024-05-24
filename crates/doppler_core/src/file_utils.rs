use conf_parser::processer::FileConf;
use doppler_parser::get_absolute_path;
use std::{
    fs::create_dir_all,
    io::Error,
    path::{Path, PathBuf},
};

pub fn copy_file(
    source_conf: &FileConf,
    destination_directory: &str,
    conf_name: &str,
) -> Result<PathBuf, Error> {
    let destination_file = format!("{}/{}", destination_directory, conf_name);
    if Path::new(destination_directory).exists() {
        return get_absolute_path(&destination_file);
    }

    create_dir_all(destination_directory)?;
    conf_parser::processer::write_to_file(source_conf, &destination_file)?;

    get_absolute_path(&destination_file)
}

pub fn create_folder(destination_directory: &str) -> Result<(), Error> {
    if Path::new(destination_directory).exists() {
        return Ok(());
    }
    create_dir_all(destination_directory)?;
    Ok(())
}
