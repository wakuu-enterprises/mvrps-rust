use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::error::Error;
use openssh::{Session, KnownHosts};

pub async fn process_segments(segments_dir: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    // Ensure the output directory exists
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Create a temporary file for the ffmpeg concat list
    let concat_file_path = Path::new(output_dir).join("concat_list.txt");
    let mut concat_file = File::create(&concat_file_path)?;

    for entry in fs::read_dir(segments_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mp4") {
            writeln!(concat_file, "file '{}'", path.display())?;
        }
    }

    // Connect to SSH server
    let session = Session::connect("user@hostname:22", KnownHosts::Strict).await?;
    let sftp = session.sftp();

    // Upload the concat list to the server
    let remote_concat_file = format!("/tmp/{}", concat_file_path.file_name().unwrap().to_str().unwrap());
    sftp.write_to(remote_concat_file.clone()).await?.write_all(&fs::read(concat_file_path)?)?;

    // Run ffmpeg command over SSH
    let output_file_path = format!("/tmp/output.mp4");
    let status = session.command("ffmpeg")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&remote_concat_file)
        .arg("-c")
        .arg("copy")
        .arg(&output_file_path)
        .status()
        .await?;

    if !status.success() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "ffmpeg command failed",
        )));
    }

    // Download the output file from the server
    sftp.read_to(output_file_path.clone()).await?.read_to_end(&mut Vec::new()).await?;
    fs::copy(output_file_path, Path::new(output_dir).join("output.mp4"))?;

    println!("Segments processed: {}", output_file_path);

    // Clean up temporary concat list file
    fs::remove_file(concat_file_path)?;

    Ok(())
}
