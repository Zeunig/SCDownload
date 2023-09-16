use std::{path::PathBuf, fs::OpenOptions, io::Write, sync::{atomic::Ordering, Mutex, Arc}, time::Duration, ffi::OsStr};
use regex::Regex;
use reqwest::blocking::Client;
use std::process::Command;
use id3::{Tag, TagLike, Version};
use crate::logging::{logging, Severities};



// While creating files, certain characters are not allowed to be in the name, so we use this to delete them
fn sanitize_song_name(input: &str) -> String {
    // idk if this part of necessary or not because all of my files are saved like this : \u0026, but better be save
    let p = Regex::new(r#"(<|>|:|"|/|\\|\||\?|\*)"#).unwrap();
    let result = p.replace_all(input, "").to_string();
    // workaround for the filename limitations like a silly specimen :P
    let result = result
    .replace("\u{0026}", "and") // & -> and
    .replace("\u{003c}3", "ily"); // <3 -> ily
    result
} 

// We always expect to get something from the regex search
#[track_caller]
fn regex_get_first(regex: Regex, text: &str) -> Option<String> {
    let e = regex.captures(text).unwrap();
    if let Some(something) = e.get(1) {
        Some(something.as_str().to_owned())
    }else {
        logging(Severities::ERROR, format!("Failed to find text \"{}\", caller : {}",text, std::panic::Location::caller()));
        None
    }
}  



struct ThreadWatcher;

// If the function panics, remove one count from tthe thread count since the thread obviously isn't running
impl Drop for ThreadWatcher {
    fn drop(&mut self) {
        if thread::panicking() {
            println!("swag"); // spoiler alert : it's not swag
            GLOBAL_THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

use std::thread;
static GLOBAL_THREAD_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);


pub fn prepare_download(songs: Vec<String>, temp_dir: &mut PathBuf, download_dir: &mut PathBuf, threads: usize, is_track: bool) {
    let max_threads = std::sync::atomic::AtomicUsize::new(0);
    max_threads.fetch_add(threads, Ordering::SeqCst);
    let req: Client = reqwest::blocking::ClientBuilder::new().use_rustls_tls().danger_accept_invalid_certs(true).build().unwrap();
    for song in songs {
        let mut run = true;
        while run {
            if GLOBAL_THREAD_COUNT.load(Ordering::SeqCst) >= max_threads.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
            } else {
                thread::sleep(Duration::from_secs(1));
                let req_wrapped = Arc::new(Mutex::new(req.clone()));
                let song_wrapped = Arc::new(Mutex::new(song.clone()));
                let temp_dir_wrapped = Arc::new(Mutex::new(temp_dir.clone()));
                let download_dir_wrapped = Arc::new(Mutex::new(download_dir.clone()));
                GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
                logging(Severities::INFO,format!("Downloading {}",&song));
                thread::spawn(move || {
                    #[allow(unused_variables)]
                    let b = ThreadWatcher;                    
                    let req_locked = req_wrapped.lock().unwrap();
                    let song_locked = song_wrapped.lock().unwrap();
                    let mut temp_dir_locked = temp_dir_wrapped.lock().unwrap();
                    let mut download_dir_locked = download_dir_wrapped.lock().unwrap();
                    
                    download(req_locked.clone(), song_locked.to_string(), &mut temp_dir_locked, &mut download_dir_locked, is_track);
                    GLOBAL_THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
                });
                run = false;
                
            }
        }
        
    }
    while GLOBAL_THREAD_COUNT.load(Ordering::Relaxed) >= 1 {
        thread::sleep(Duration::from_secs(1));
    }
}


// If cache is found, we count the amount of files there and add them together as audio0.mp3, audio1.mp3, audio2.mp3 etc.. Doing mp3cat *.mp3 wouldn't work because it would concat those files like this: audio0.mp3 audio1.mp3 audio10.mp3..
fn count_mp3(root: PathBuf) -> u32 {
    let mut result = vec![];

    for path in std::fs::read_dir(root).unwrap() {
        let path = path.unwrap().path();
        if let Some("mp3") = path.extension().and_then(OsStr::to_str) {
            result.push(path.to_owned());
        }
    }
    result.len() as u32
}


fn download(req: Client, song: String, temp_dir: &mut PathBuf, download_dir: &mut PathBuf, is_track: bool) {
    let mut temp_dir = temp_dir.clone().to_owned();
    let mut download_dir = download_dir.clone().to_owned();
    let mut audio_file_nmbr_count: u32 = 0;
    temp_dir.push(song.split("/").nth(0).unwrap());
    temp_dir.push(song.split("/").nth(1).unwrap());
    if is_track {
        download_dir.push(song.split("/").nth(0).unwrap());
    }
    
    match std::fs::create_dir_all(&temp_dir) {
        Ok(_) => {},
        Err(err) => {
            println!("Failed to create directory, additional information : {}",err);
        }
    }
    
    match std::fs::create_dir_all(&download_dir) {
        Ok(_) => {},
        Err(err) => {
            println!("Failed to create directory, additional information : {}",err);
        }
    }
    let mut temp = temp_dir.clone();temp.push("audio0.mp3");
    let mut cover_path = temp_dir.clone();cover_path.push("cover.jpg");
    #[allow(unused_assignments)]
    let mut artist: String = String::new();
    #[allow(unused_assignments)]
    let mut song_name: String = String::new();
    #[allow(unused_assignments)]
    let mut cover: String = String::new();
    // CACHE
    if temp.exists() {
        logging(Severities::INFO, format!("Song already exists in cache : {}",song));
        audio_file_nmbr_count = count_mp3(temp_dir.clone());
        drop(temp);
        let mut temp = temp_dir.clone();temp.push("metadata.txt");
        let metadata = std::fs::read_to_string(temp);
        match metadata {
            Ok(metadata) => {
                let metadata: Vec<&str> = metadata.split("|").collect();
                artist = metadata.get(0).unwrap().to_string();
                song_name = metadata.get(1).unwrap().to_string();
                //cover = metadata.get(2).unwrap().to_string();
            },
            Err(_) => {
                logging(Severities::INFO,"Found song in cache, but not metadata");
                let r = req.get(format!("https://soundcloud.com/{song}"))
                .send().unwrap().text().unwrap();
                // ADDITIONAL INFORMATION PARSING BEGIN
                artist = regex_get_first(Regex::new(r#""username":"(.*?)""#).unwrap(), &r).unwrap();
                song_name = regex_get_first(Regex::new(r#""title":"(.*?)""#).unwrap(), &r).unwrap();
                // ALBUM COVER PARSING
                // This either return the profile picture of the artist (if no cover is specified) or the cover image itself, or neither because the artist doesn't have a profile picture either
                cover = match regex_get_first(
                    Regex::new(r#"<meta property="og:image" content="(.*?)""#).unwrap(), &r)
                    {
                        Some(e) => {
                            e
                        },
                        None => {
                            String::from("None")
                        }
                    };
                {
                    let mut temp = temp_dir.clone();temp.push("metadata.txt");
                    let mut file = OpenOptions::new().write(true).create(true).open(temp).unwrap();
                    let _ = file.write_all(format!("{}|{}|{}",artist,song_name,cover).as_bytes());
                }
                if !cover_path.exists() {
                    if !cover.contains("None") {
                        let r = req.get(cover).send().unwrap().bytes().unwrap();
                        {
                            let mut temp = temp_dir.clone();temp.push("cover.jpg");
                            let mut file = OpenOptions::new().write(true).create(true).open(temp).unwrap();
                            let _ = file.write_all(&r);
                        }
                    }
                    
                }
                
                
            }
        }
    }else {
        drop(temp);
        let r = req.get(format!("https://soundcloud.com/{song}"))
        .send().unwrap().text().unwrap();
        // ADDITIONAL INFORMATION PARSING BEGIN
        artist = regex_get_first(Regex::new(r#""username":"(.*?)""#).unwrap(), &r).unwrap();
        song_name = regex_get_first(Regex::new(r#""title":"(.*?)""#).unwrap(), &r).unwrap();
        // ALBUM COVER PARSING
        cover = match regex_get_first(
            Regex::new(r#"<meta property="og:image" content="(.*?)""#).unwrap(), &r)
            {
                Some(e) => {
                    e
                },
                None => {
                    String::from("None")
                }
            };
        if !cover_path.exists() {
            let re = req.get(&cover).send().unwrap().bytes().unwrap();
            {
                let mut temp = temp_dir.clone();temp.push("cover.jpg");
                let mut file = OpenOptions::new().write(true).create(true).open(temp).unwrap();
                let _ = file.write_all(&re);
            }
        }
        if !cover.contains("None") {
            let r = req.get(cover).send().unwrap().bytes().unwrap();
            {
                let mut temp = temp_dir.clone();temp.push("cover.jpg");
                let mut file = OpenOptions::new().write(true).create(true).open(temp).unwrap();
                let _ = file.write_all(&r);
            }
        }
        let re = Regex::new(r#"track_authorization":"(.*?)""#).unwrap();
        let capture = re.captures(&r).unwrap();
        if let Some(track_auth) = capture.get(1) {
            let capture = Regex::new(r#"\{"url":"(.*?)""#).unwrap().captures(&r).unwrap();
            if let Some(hls) = capture.get(1) {
                let track_auth = track_auth.as_str();
                let hls = hls.as_str();
                let r = req.get(format!("{hls}?client_id=baLbCx2miy7TG4nunX9yTWklG3ecgeE9&track_authorization={track_auth}"))
                .send().unwrap();
                if !r.status().is_success() {
                    logging(Severities::ERROR, format!("Expected status code 200, got status code {} on song : {}",r.status(),&song));
                    return;
                }
                let r = r.text().unwrap();
                if r.contains(r#""url":null"#) {
                    logging(Severities::ERROR, format!("No download link found on song : {} | If this issue persists, please contact the developer",&song));
                    return;
                }
                let r = req.get(&r[8..r.len()-2]).send().unwrap().text().unwrap();
                let re = Regex::new(r#"(https://cf-hls-media.sndcdn.com/media/.*?)\n"#).unwrap();
                let links = re.captures_iter(&r);
                
                
                for link in links {
                    let link = &link[0];
                    let r = req.get(link).send().unwrap().bytes().unwrap();
                    let mut a = temp_dir.clone();
                    a.push(format!("audio{}.mp3",audio_file_nmbr_count));
                    let mut file = OpenOptions::new().write(true).create(true).open(a).unwrap();
                    let a = file.write_all(&r);
                    match a {
                        Ok(_) => {},
                        Err(err) => {
                            println!("Failed to write to file, additional information : {}",err);
                        }
                    }
                    audio_file_nmbr_count += 1;
                }
            }
        }
    }
    // mp3cat magic
    let mut arguments: Vec<String> = Vec::new();
    download_dir.push(format!("{}.mp3",sanitize_song_name(&song_name)));
    
    let mut audio = 0;
    while audio < audio_file_nmbr_count {
        let mut a = temp_dir.clone();
        a.push(format!("audio{}.mp3", audio));
        arguments.push(a.to_str().unwrap().to_string());
        audio = audio + 1;
    }
    
    let mut command = Command::new("mp3cat")
    //.arg(download_dir.to_str().unwrap())
    .args(arguments)
    .arg("-o")
    .arg(download_dir.to_str().unwrap())
    .arg("-q")
    .arg("-f")
    .spawn().expect("Failed to execute cmd message");
    match command.wait() {
        Ok(_) => {
            let mut tag: Tag = match Tag::read_from_path(download_dir.to_str().unwrap()) {
                Ok(tag) => tag,
                Err(id3::Error{kind: id3::ErrorKind::NoTag, ..}) => Tag::new(),
                Err(_) => return,
            };
            // add cover image, artist etc. to song
            tag.set_title(song_name);
            tag.set_album_artist(&artist);
            tag.set_artist(artist);
            // Every album must be unique, because of Spotify's weird optimization(?) of using one image for the album
            tag.set_album(&song);
            let mut cover_image = temp_dir.clone(); cover_image.push("cover.jpg");
            tag.add_frame(id3::frame::Picture { 
                mime_type: String::from("image/jpeg"), 
                picture_type: id3::frame::PictureType::Media,
                description: String::new(),
                data: std::fs::read(cover_image).unwrap()
            });
            let _ = tag.write_to_path(download_dir.to_str().unwrap(), Version::Id3v23);
        },
        Err(_) => {
            logging(Severities::WARNING, "Error occured while running command, skipping ID3 tags");
        }
    } 
    logging(Severities::INFO, format!("Finished downloading {}",song));
}