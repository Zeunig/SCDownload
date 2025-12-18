use std::{ffi::OsStr, fs::OpenOptions, io::{Read, Write}, path::PathBuf, sync::{atomic::Ordering, Arc, Mutex}, time::Duration};
use regex::Regex;
use reqwest::blocking::Client;
use id3::{Tag, TagLike, Version};
use crate::{logging::{logging, Severities}, Arguments};
use reqwest::header::HeaderMap;
use ffmpeg_sidecar::command::FfmpegCommand;

// While creating files, certain characters are not allowed to be in the name, so we use this to delete them
fn sanitize_song_name(input: &str) -> String {
    let mut result = input
    .replace("\\u0026", "and"); // & -> and
    result = result.replace("\\u003c3", "ily"); // <3 -> ily
    // idk if this part of necessary or not because all of my files are saved like this : \u0026, but better be save
    let p = Regex::new(r#"(<|>|:|"|/|\\|\||\?|\*)"#).unwrap();
    let result = p.replace_all(&result, "").to_string();
    // workaround for the filename limitations like a silly specimen :P
    
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

// If the function panics, remove one count from the thread count since the thread obviously isn't running
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


pub fn prepare_download(songs: Vec<String>, arguments: Arguments, is_track: bool, client_id: String) {
    
    let max_threads = std::sync::atomic::AtomicUsize::new(0);
    if songs.len() == 1 {
        max_threads.fetch_add(1, Ordering::SeqCst);
    }else {
        max_threads.fetch_add(arguments.thread_count, Ordering::SeqCst);
    }
    let req: Client = reqwest::blocking::ClientBuilder::new().use_rustls_tls().danger_accept_invalid_certs(true).build().unwrap();
    for song in songs {
        let mut run = true;
        while run {
            if GLOBAL_THREAD_COUNT.load(Ordering::SeqCst) >= max_threads.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
            } else {
                let req_wrapped = Arc::new(Mutex::new(req.clone()));
                let song_wrapped = Arc::new(Mutex::new(song.clone()));
                /*let temp_dir_wrapped = Arc::new(Mutex::new(arguments.temp_dir.clone()));
                let download_dir_wrapped = Arc::new(Mutex::new(arguments.download_dir.clone()));*/
                let arguments_wrapped = Arc::new(Mutex::new(arguments.clone()));
                let client_id_wrapped = Arc::new(Mutex::new(client_id.clone()));
                GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
                logging(Severities::INFO,format!("Downloading {}",&song));
                thread::spawn(move || {
                    #[allow(unused_variables)]
                    let b = ThreadWatcher;                    
                    let req_locked = req_wrapped.lock().unwrap();
                    let song_locked = song_wrapped.lock().unwrap();
                    /*let mut temp_dir_locked = temp_dir_wrapped.lock().unwrap();
                    let mut download_dir_locked = download_dir_wrapped.lock().unwrap();*/
                    let arguments_locked = arguments_wrapped.lock().unwrap();
                    let client_id_locked = client_id_wrapped.lock().unwrap();
                    
                    download(req_locked.clone(), song_locked.to_string(), &arguments_locked , is_track, &client_id_locked);
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
        if let Some(x) = path.extension().and_then(OsStr::to_str) {
            if x == "mp3" || x == "m4s" {
                result.push(path.to_owned());
            }
        }
    }
    result.len() as u32
}

enum FileType {
    Undefined,
    MP3,
    M4S
}
#[derive(Default, Clone, Debug)]
struct Song {
    audio_file_count: u32,
    uri: String,
    cover_path: PathBuf,
    artist: String,
    name: String,
    cover: String
}

fn download(req: Client, song_uri: String, arguments: &Arguments, is_track: bool, client_id: &str) {
    let mut temp_dir = arguments.temp_dir.clone().to_owned();
    let mut download_dir = arguments.download_dir.clone().to_owned();
    let mut file_type: FileType = FileType::MP3;
    temp_dir.push(song_uri.split("/").nth(0).unwrap());
    temp_dir.push(song_uri.split("/").nth(1).unwrap());
    if is_track {
        download_dir.push(song_uri.split("/").nth(0).unwrap());
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
    let mut temp = temp_dir.clone();temp.push("0.mp3");
    let mut song = Song::default();
    song.cover_path = {
        let mut patthh = temp_dir.clone();
        patthh.push("cover.jpg");
        patthh
    };
    song.uri = song_uri;
    // CACHE
    if !arguments.disable_cache {
        if temp.exists() {
            logging(Severities::INFO, format!("Song already exists in cache : {}",song.name));
            song.audio_file_count = count_mp3(temp_dir.clone());
            drop(temp);
            let mut temp = temp_dir.clone();temp.push("metadata.txt");
            let metadata = std::fs::read_to_string(temp);
            match metadata {
                Ok(metadata) => {
                    let metadata: Vec<&str> = metadata.split("|").collect();
                    song.artist = metadata.get(0).unwrap().to_string();
                    song.name = metadata.get(1).unwrap().to_string();
                },
                Err(_) => {
                    download_metadata(req, &song.uri, arguments, &mut temp_dir, &mut song.artist, &mut song.name, &mut song.cover);
                }
            }
        }else {
            download_audio(&mut song, arguments, &req, &mut temp_dir, client_id);
        }
    }else {
        download_audio(&mut song, arguments, &req, &mut temp_dir, client_id);
    }
    // check if we have legacy or non-legacy streaming downloaded
    let mut check = temp_dir.clone();
    check.push("1.m4s");
    if check.exists() {
        file_type = FileType::M4S;
    }

    match file_type {
        FileType::MP3 => {
            logging(Severities::DEBUG, format!("Downloading {} with file format MP3", song.name));
            download_dir.push(format!("{}.mp3",sanitize_song_name(&song.name)));
            let mut input_string = "concat:".to_string();
            for i in 0..song.audio_file_count {
                if i > 0 {
                    input_string += "|";
                }
                //input_string += &format!("/{}.mp3",i);
                let mut a = temp_dir.clone();
                a.push(format!("{}.mp3", i));
                input_string += a.to_str().unwrap();
            }
            logging(Severities::DEBUG, format!("Input string : {}",input_string));
            let mut command = FfmpegCommand::new();
            command.input(input_string);
            command.codec_audio("copy");
            command.output(download_dir.to_str().unwrap());
            logging(Severities::DEBUG, format!("{:?}",command));
            let mut command = command.spawn()
                    .unwrap();
            let command = command.wait()
                    .unwrap();
            logging(Severities::DEBUG, format!("{:?}",command));
            add_metadata(song, &mut temp_dir, &download_dir);
        }
        FileType::M4S => {
            logging(Severities::DEBUG, format!("Downloading {} with file format M4S", song.name));
            let mut buffer = Vec::new();
            let mut path = temp_dir.clone();
            path.push("0.mp3");
            let mut oo = OpenOptions::new().read(true).open(path).unwrap();
            oo.read_to_end(&mut buffer).unwrap();
            for i in 1..song.audio_file_count {
                let mut path = temp_dir.clone();
                path.push(format!("{i}.m4s"));
                let mut oo = OpenOptions::new().read(true).open(path).unwrap();
                oo.read_to_end(&mut buffer).unwrap();
            }
            download_dir.push(format!("temp_{}.mp4", sanitize_song_name(&song.name)));
            let mut oo = OpenOptions::new().write(true).create(true).truncate(true).open(&download_dir).unwrap();
            oo.write_all(&buffer).unwrap();
            let mut convert = FfmpegCommand::new();
            convert.input(download_dir.to_str().unwrap());
            download_dir.pop();
            download_dir.push(format!("{}.mp3",sanitize_song_name(&song.name)));
            convert.output(download_dir.to_str().unwrap());
            convert.spawn().unwrap().wait().unwrap();
            let final_dir = download_dir.clone();
            download_dir.pop();
            download_dir.push(format!("temp_{}.mp4", sanitize_song_name(&song.name)));
            if let Err(_) = std::fs::remove_file(&download_dir) {
                logging(Severities::ERROR, format!("Failed to delete temporary file at path : {:?}",&download_dir.to_str()));
            }
            add_metadata(song, &mut temp_dir, &final_dir);
            },
        FileType::Undefined => {
            logging(Severities::ERROR, format!("Something went wrong while trying to download song : {} | If this issue persists, please contact the developer", song.name));
        },
    }
    
    
}

fn download_metadata(req: Client, song: &str, arguments: &Arguments, temp_dir: &mut PathBuf, artist: &mut String, song_name: &mut String, cover: &mut String) {
    let mut headers = HeaderMap::new();
    let mut cover_path = temp_dir.clone();cover_path.push("cover.jpg");

    headers.insert("Accept", "application/json, text/javascript, /*/; q=0.1".parse().unwrap());
    headers.insert("Accept-Language", "hu-HU,hu;q=0.9".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Origin", "https://soundcloud.com".parse().unwrap());
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Referer", "https://soundcloud.com/".parse().unwrap());
    headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("sec-ch-ua", "\"Chromium\";v=\"116\", \"Not)A;Brand\";v=\"24\", \"Google Chrome\";v=\"116\"".parse().unwrap());
    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
    headers.insert("sec-ch-ua-platform", "\"Windows\"".parse().unwrap());
    let r = req.get(format!("https://soundcloud.com/{song}"))
    .headers(headers).send().unwrap().text().unwrap();
    // ADDITIONAL INFORMATION PARSING BEGIN
    *artist = regex_get_first(Regex::new(r#""username":"(.*?)""#).unwrap(), &r).unwrap();
    *song_name = regex_get_first(Regex::new(r#""title":"(.*?)""#).unwrap(), &r).unwrap();
    // ALBUM COVER PARSING
    // This either return the profile picture of the artist (if no cover is specified) or the cover image itself, or neither because the artist doesn't have a profile picture either
    *cover = match regex_get_first(
        Regex::new(r#"<meta property="og:image" content="(.*?)""#).unwrap(), &r)
        {
            Some(e) => {
                if arguments.original_cover_image {
                    e.replace("t500x500", "original")
                }else {
                    e
                }
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
    if !cover_path.exists() && !cover.contains("None") {
        let r = req.get(cover.to_string()).send().unwrap().bytes().unwrap();
        {
            let mut temp = temp_dir.clone();temp.push("cover.jpg");
            let mut file = OpenOptions::new().write(true).create(true).open(temp).unwrap();
            let _ = file.write_all(&r);
        }
    }
}

fn add_metadata(song: Song, temp_dir: &mut PathBuf, download_dir: &PathBuf) {
    let mut tag: Tag = match Tag::read_from_path(download_dir.to_str().unwrap()) {
        Ok(tag) => tag,
        Err(id3::Error{kind: id3::ErrorKind::NoTag, ..}) => Tag::new(),
        Err(_) => return,
    };
    // add cover image, artist etc. to song
    tag.set_title(song.name);
    tag.set_album_artist(&song.artist);
    tag.set_artist(song.artist);
    // Every album must be unique, because of Spotify's weird optimization(?) of using one image for the album
    tag.set_album(&song.uri);
    let mut cover_image = temp_dir.clone(); cover_image.push("cover.jpg");
    tag.add_frame(id3::frame::Picture { 
        mime_type: String::from("image/jpeg"), 
        picture_type: id3::frame::PictureType::Media,
        description: String::new(),
        data: std::fs::read(cover_image).unwrap()
    });
    let _ = tag.write_to_path(download_dir.to_str().unwrap(), Version::Id3v23);
    logging(Severities::INFO, format!("Successfully downloaded {}",song.uri));
}

fn download_audio(song: &mut Song, arguments: &Arguments, req: &Client, temp_dir: &mut PathBuf, client_id: &str) -> FileType {
    let mut file_type = FileType::Undefined;
    download_metadata(req.clone(), &song.uri, arguments, temp_dir, &mut song.artist, &mut song.name, &mut song.cover);
    let re = Regex::new(r#"track_authorization":"(.*?)""#).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json, text/javascript, */*; q=0.1".parse().unwrap());
    headers.insert("Accept-Language", "hu-HU,hu;q=0.9".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Origin", "https://soundcloud.com".parse().unwrap());
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Referer", "https://soundcloud.com/".parse().unwrap());
    headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("sec-ch-ua", "\"Chromium\";v=\"116\", \"Not)A;Brand\";v=\"24\", \"Google Chrome\";v=\"116\"".parse().unwrap());
    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
    headers.insert("sec-ch-ua-platform", "\"Windows\"".parse().unwrap());
    let r = req.get(format!("https://soundcloud.com/{}",song.uri))
    .headers(headers).send().unwrap().text().unwrap();
    let capture = re.captures(&r).unwrap();
    if let Some(track_auth) = capture.get(1) {
        for (_, [hls]) in Regex::new(r#"\{"url":"(.*?)""#).unwrap().captures_iter(&r).map(|x| x.extract()) {
            let track_auth = track_auth.as_str();
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "*/*".parse().unwrap());
            headers.insert("Accept-Language", "hu-HU,hu;q=0.9".parse().unwrap());
            headers.insert("Cache-Control", "no-cache".parse().unwrap());
            headers.insert("Connection", "keep-alive".parse().unwrap());
            headers.insert("Content-Type", "application/json".parse().unwrap());
            headers.insert("Origin", "https://soundcloud.com".parse().unwrap());
            headers.insert("Pragma", "no-cache".parse().unwrap());
            headers.insert("Referer", "https://soundcloud.com/".parse().unwrap());
            headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
            headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
            headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
            headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36".parse().unwrap());
            headers.insert("sec-ch-ua", "\"Chromium\";v=\"116\", \"Not)A;Brand\";v=\"24\", \"Google Chrome\";v=\"116\"".parse().unwrap());
            headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
            headers.insert("sec-ch-ua-platform", "\"Windows\"".parse().unwrap());
            logging(Severities::DEBUG, format!("Trying HLS link : {hls}?client_id={client_id}&track_authorization={track_auth}"));
            let r = req.get(format!("{hls}?client_id={client_id}&track_authorization={track_auth}"))
            .headers(headers.clone())
            .send().unwrap();
            if !r.status().is_success() {
                logging(Severities::WARNING, format!("Expected status code 200, got status code {} on song, trying next download method : {}",r.status(),&song.name));
                continue;
            }
            let r = r.text().unwrap();
            if r.contains(r#""url":null"#) {
                logging(Severities::WARNING, format!("No download link found on song, trying next download method : {}",&song.name));
                continue;
            }
            let url_re = Regex::new(r#"\{"url":"(.*?)".*?}"#).unwrap();
            let url_captures = url_re.captures(&r);
            let mut url;
            if let Some(url_captured_some) = url_captures {
                url = url_captured_some.get(1).unwrap().as_str();
            }else {
                url = &r[8..r.len()-2];
            }
            logging(Severities::DEBUG, format!("Parsed URL : {}", url));
            let mut r = req.get(url).headers(headers.clone()).send().unwrap().text().unwrap();
            if r.contains("Forbidden") {
                url = &r[8..r.len()-19];
                r = req.get(url).headers(headers.clone()).send().unwrap().text().unwrap();
            }
            logging(Severities::INFO, format!("Valid download URL found, downloading song {}",song.name));
            let re = Regex::new(r#"(https://cf-hls-media.sndcdn.com/media/.*?|https://playback.media-streaming.soundcloud.cloud.*?)\n"#).unwrap();
            let links = re.captures_iter(&r);
            file_type = FileType::MP3;
            for link in links {
                let link = &link[0].replace("\"","");
                let r = req.get(link).headers(headers.clone()).send().unwrap().bytes().unwrap();
                let mut a = temp_dir.clone();
                if link.contains(".m4s") {
                    a.push(format!("{}.m4s",song.audio_file_count));
                    file_type = FileType::M4S;
                }else {
                    a.push(format!("{}.mp3",song.audio_file_count));
                }
                let mut file = OpenOptions::new().write(true).create(true).open(a).unwrap();
                let a = file.write_all(&r);
                match a {
                    Ok(_) => {},
                    Err(err) => {
                        println!("Failed to write to file, additional information : {}",err);
                    }
                }
                song.audio_file_count += 1;
            }
            break;
        }
    }
    file_type
}