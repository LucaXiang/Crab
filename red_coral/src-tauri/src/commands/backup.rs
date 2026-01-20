use std::fs::{self, File};
use std::io::{Read, Write};
use tauri::{AppHandle, Manager};
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::core::ApiResponse;

fn zip_dir(
    it: &mut std::fs::ReadDir,
    prefix: &str,
    writer: &mut ZipWriter<File>,
    options: FileOptions<()>,
) -> zip::result::ZipResult<()> {
    for entry in it {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid file name",
            ))?
            .to_str()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8 name",
            ))?;
        let path_as_string = format!("{}{}", prefix, name);

        if path.is_dir() {
            writer.add_directory(&path_as_string, options)?;
            let mut dir_it = fs::read_dir(&path)?;
            zip_dir(
                &mut dir_it,
                &format!("{}/", path_as_string),
                writer,
                options,
            )?;
        } else {
            writer.start_file(&path_as_string, options)?;
            let mut f = File::open(&path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            writer.write_all(&buffer)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn export_data(app: AppHandle, path: String) -> Result<ApiResponse<()>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Backup database directory
    let db_dir = app_dir.join("database");
    if db_dir.exists() {
        if let Ok(mut it) = fs::read_dir(&db_dir) {
            zip.add_directory("database/", options)
                .map_err(|e| e.to_string())?;
            zip_dir(&mut it, "database/", &mut zip, options).map_err(|e| e.to_string())?;
        }
    }

    // Backup config.json
    let config_file = app_dir.join("config.json");
    if config_file.exists() {
        zip.start_file("config.json", options)
            .map_err(|e| e.to_string())?;
        let mut f = File::open(config_file).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
    }

    // Backup tenants directory
    let tenants_dir = app_dir.join("tenants");
    if tenants_dir.exists() {
        if let Ok(mut it) = fs::read_dir(&tenants_dir) {
            zip.add_directory("tenants/", options)
                .map_err(|e| e.to_string())?;
            zip_dir(&mut it, "tenants/", &mut zip, options).map_err(|e| e.to_string())?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish zip: {}", e))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn import_data(app: AppHandle, path: String) -> Result<ApiResponse<()>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match file.enclosed_name() {
            Some(path) => app_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(ApiResponse::success(()))
}
