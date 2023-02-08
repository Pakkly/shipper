use crate::common;
use crate::common::InstallProgressSegment;
use crate::fslog;
use log::debug;
use pakkly_error::FormattedError;
use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::PathBuf;

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn is_file_zip(path: &PathBuf) -> Result<bool, FormattedError> {
    let mut file = fslog::file_open(&path)?;
    let mut buf: [u8; 4] = [0; 4];
    file.read_exact(&mut buf)?;
    let zip_magic = [0x50, 0x4B, 0x03, 0x04];
    return Ok(zip_magic == buf);
}
pub fn extract<F>(filename: &PathBuf, target_dir: &PathBuf, progress_cb: F) -> Result<Vec<String>, FormattedError>
where
    F: Fn(f32, InstallProgressSegment),
{
    debug!("Opening zip: {:?}", filename);
    trim_zip_postfix(&filename)?;
    let file = fslog::file_open(&filename)?;

    let mut archive = zip::ZipArchive::new(file)?;
    let mut dst_files: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        progress_cb(((i as f64) / (archive.len() as f64) * 0.5) as f32, InstallProgressSegment::Installing);
        let mut file = archive.by_index(i)?;
        let outpath_raw = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let mut outpath = PathBuf::new();
        outpath.push(target_dir);
        outpath.push(outpath_raw);

        if (&*file.name()).ends_with('/') {
            fslog::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !fslog::exists(&p) {
                    fslog::create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            dst_files.push(outpath.to_str().unwrap().to_owned());
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    drop(archive);
    fslog::remove_file(&filename)?;
    return Ok(dst_files);
}

fn trim_zip_postfix(zip_path: &PathBuf) -> Result<(), FormattedError> {
    //removes all data after the zip comments.
    let mut zip = std::fs::OpenOptions::new().read(true).write(true).truncate(false).create(true).open(zip_path)?;
    let mut buffer: Vec<u8> = Vec::new();
    buffer.resize(1024 * 1024, 0x00);
    buffer.fill(0x00);
    let mut offset = 0;

    let mut end_signature_index = 0;
    let zip_end_signature = [0x50, 0x4b, 0x05, 0x06];
    'mainloop: loop {
        if offset == 0 {
            //double check magic
            let mut buf: [u8; 4] = [0; 4];
            zip.read_exact(&mut buf)?;
            let zip_magic = [0x50, 0x4B, 0x03, 0x04];
            if buf != zip_magic {
                return Err(FormattedError::from_str("Zip magic not detected!".to_string()));
            }
            offset = 4;
        }

        let length = zip.read(buffer.as_mut_slice())?;
        if length == 0 {
            let e = format!("End of zip not found! Malformed zip. Read {} bytes.", offset + length);
            common::submit_basic_crash(e.as_str());
            return Err(FormattedError::from_str(e.to_string()));
        }
        let slice = buffer[0..length].to_vec();
        for i in 0..slice.len() {
            if slice[i] == zip_end_signature[end_signature_index] {
                end_signature_index += 1; //check the next byte as well
            } else {
                end_signature_index = 0; //reset the search!
            }
            if end_signature_index == zip_end_signature.len() {
                //found the ZIP End of central directory record
                offset += i;
                break 'mainloop;
            }
        }
        offset += length;
    }

    let mut comment_length_buf: [u8; 2] = [0; 2];
    let comment_buf_offset = 17 + u64::try_from(offset).unwrap();
    zip.seek(SeekFrom::Start(comment_buf_offset))?;
    zip.read_exact(&mut comment_length_buf)?;
    let comment_length = u16::from_le_bytes(comment_length_buf);

    let trimmed_size = comment_buf_offset + 2 + u64::from(comment_length);
    zip.set_len(trimmed_size)?;
    drop(zip);

    Ok(())
}
