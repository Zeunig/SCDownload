use std::{env, process::exit, path::{Path, PathBuf}};

use download::prepare_download;

use crate::logging::logging;
mod download;
mod logging;

fn additional_argument_helper(args: &Vec<String>) -> (PathBuf,PathBuf) {
    let mut temp_dir: PathBuf = env::temp_dir();
    let mut download_dir: PathBuf = Path::new("./").to_path_buf();
    if let Some(tempdir_argument) = args.iter().find(|&x| x.contains("--temp-dir")) {
        if let Some(equal) = tempdir_argument.find("=") {
            temp_dir = PathBuf::from(&tempdir_argument[equal+1..]);
            if !(Path::is_dir(Path::new(&temp_dir))) {
                println!(r#"Invalid temp_dir, using default option"#);temp_dir = env::temp_dir();
            }
        }
    }
    if let Some(downloadir_argument) = args.iter().find(|&x| x.contains("--download-dir")) {
        if let Some(equal) = downloadir_argument.find("=") {
            download_dir = PathBuf::from(&downloadir_argument[equal+1..]);
            if !(Path::is_dir(Path::new(&download_dir))) {
                println!(r#"Invalid download_dir, using default option"#);download_dir = Path::new("./").to_path_buf();
            }
        }
    }
    (temp_dir,download_dir)
}

fn check_for_invalid_arguments(args: &Vec<String>) {
    if args.len() == 1 {
        println!(r#"SCDownload - Made by Zeunig

This software allows you to easily download tracks, albums and playlists from SoundCloud into your computer.
Usage:
scdownload.exe <track/album/playlist> <id of the track/album/playlist>
Additional arguments:
--temp-dir="path" - Sets the temporary folder location
--download-dir="path" - Sets the download folder location"#);
        exit(0);
    }
    if args.len() == 2 {
        match args.get(1).unwrap().as_str() {
            "track" => {
                println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected track ID:
example: odcodone/lp-printer"#);exit(0);
            },
            "album" => {
                println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected album ID:
example: ossianofficial/sets/best-of-1998-2008"#);exit(0);
            },
            "playlist" => {
                println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected playlist ID:
example: zeunig/sets/hardstyle"#);exit(0);
            },
            "artist" => {
                println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected artist' username:
example: zeunig"#);exit(0);
            }

            _ => {
                println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected valid type:
scdownload.exe <track/album/playlist> <id of the track/album/playlist>"#);exit(0);
            }
        }
    }
    let first = args.get(1).unwrap();
    if !(first.contains("track")) && !(first.contains("album")) && !(first.contains("playlist")) && !(first.contains("artist")) {
        println!(r#"SCDownload - Made by Zeunig

Invalid usage, expected either album/playlist/track/artist as first argument"#);exit(0);
    }
}

fn trimming(track: String) -> String {
    let splitted: Vec<&str> = track.split("https://soundcloud.com/").collect();
    let track: String = splitted.get(splitted.len()-1).unwrap().to_string();
    let splitted: Vec<&str> = track.split("?").collect();
    splitted.get(0).unwrap().to_string()
}

fn playlist_to_vec(req: reqwest::blocking::Client, dest: &mut Vec<String>, orig: Vec<String>) {
    // orig -> track ids
    use reqwest::header::HeaderMap;
    use regex::Regex;
    let reg = Regex::new(r#"permalink_url":"https://soundcloud\.com/((?:[^"/]*?)/(?:[^"/]*?))","#).unwrap();
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
    let mut original = orig;
    let mut url: String = String::from("https://api-v2.soundcloud.com/tracks?ids=");
    let mut temp = 0;
    while original.len() > 0 {
        url.push_str(&format!("{}%2C",original.get(0).unwrap()));
        original.remove(0);
        temp = temp + 1;
        if temp == 10 {
            url.push_str("&client_id=0nr4Ys43jAqfn0VkGXfxTWh9d4NB0o54&[object Object]=&app_version=1694501791&app_locale=en");
            let r = req.get(url).headers(headers.clone()).send().unwrap().text().unwrap();
            for capture in reg.captures_iter(&r).map(|c| c.get(1)) {
                dest.push(capture.unwrap().as_str().to_string());
            }
            url = String::from("https://api-v2.soundcloud.com/tracks?ids=");
            temp = 0;
        }
    }
    url.push_str("&client_id=0nr4Ys43jAqfn0VkGXfxTWh9d4NB0o54&[object Object]=&app_version=1694501791&app_locale=en");
    let r = req.get(url).headers(headers.clone()).send().unwrap().text().unwrap();
    for capture in reg.captures_iter(&r).map(|c| c.get(1)) {
        dest.push(capture.unwrap().as_str().to_string());
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    check_for_invalid_arguments(&args);
    let mut paths = additional_argument_helper(&args);
    paths.0.push("SCDownloader");
    // We're safe the unwrap the args because we checked if the argument list of valid
    match args.get(1).unwrap().as_str() {
        "track" => {
            let mut arg2 = args.get(2).unwrap().to_owned();
            if arg2.contains("soundcloud.com/") {
                let p: Vec<&str> = arg2.split("soundcloud.com/").collect();
                arg2 = p[1].to_string();
            }
            {
                let p: Vec<&str> = arg2.split("?").collect(); // we don't need anything after the ?
                arg2 = p[0].to_string();
            }
            let mut list: Vec<String> = Vec::new();
            list.push(trimming(arg2));
            prepare_download(list, &mut paths.0, &mut paths.1, 1, true);
        },
        "playlist" | "album" => {
            let mut arg2 = args.get(2).unwrap().to_owned();
            if args.get(2).unwrap().contains("https://soundcloud.com/") {
                let p: Vec<&str> = arg2.split("https://soundcloud.com/").collect();
                arg2 = p[1].to_string();
            }
            logging(logging::Severities::INFO, "Fetching playlist");
            paths.1.push(format!("{}",arg2));
            use reqwest::header::HeaderMap;
            use regex::Regex;
            let mut list: Vec<String> = Vec::new();
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
            let req = reqwest::blocking::ClientBuilder::new().use_rustls_tls().danger_accept_invalid_certs(true).build().unwrap();
            let r = req.get(format!("https://soundcloud.com/{}", arg2)).send().unwrap().text().unwrap();
            let reg = Regex::new(r#""id":([0-9]*?),"kind":"track","#).unwrap();
            for a in reg.captures_iter(&r).map(|c| c.get(1)) {
                match a {
                    Some(e) => {
                        list.push(e.as_str().to_string());
                    },
                    None => {}
                }
            }
            let mut songs: Vec<String> = Vec::new();
            playlist_to_vec(req, &mut songs, list);
            prepare_download(songs, &mut paths.0, &mut paths.1, 3, false);
        },
        "artist" => {
            use regex::Regex;
            {
                println!("Disclaimer, this feature is made for artists to back up their songs in case they lost them, if you're trying to download another artist's song, please ask for their permission\nPress ENTER to proceed");
                let mut input_text = String::new();
                std::io::stdin()
                .read_line(&mut input_text)
                .expect("failed to read from stdin");
            }
            logging(logging::Severities::INFO, "Fetching songs");
            let mut arg2 = args.get(2).unwrap().to_owned();
            if arg2.contains("soundcloud.com/") {
                let n: Vec<&str> = arg2.split("soundcloud.com/").collect();
                arg2 = n[1].to_owned();
            }
            paths.1.push(format!("artist/{}",arg2));
            let req = reqwest::blocking::ClientBuilder::new().use_rustls_tls().danger_accept_invalid_certs(true).build().unwrap();
            let r = req.get(format!("https://soundcloud.com/{}",arg2)).send().unwrap().text().unwrap();
            let reg = Regex::new(r#"content="soundcloud://users:([0-9]*?)""#).unwrap();
            let uid = reg.captures(&r).unwrap().get(1).unwrap().as_str().to_owned();
            let r = req.get(format!("https://api-v2.soundcloud.com/users/{}/tracks?offset=0&limit=79999&representation=&client_id=TtbhBUaHqao06g1mUwVTxbjj8TSUkiCl&app_version=1694761046&app_locale=en",uid)).send().unwrap().text().unwrap();
            let reg = Regex::new(r#""permalink_url":"https://soundcloud\.com/((?:[a-zA-Z0-9-_]*?)/(?:[a-zA-Z0-9-_]*?))""#).unwrap();
            let mut list: Vec<String> = Vec::new();
            for a in reg.captures_iter(&r).map(|c| c.get(1)) {
                match a {
                    Some(e) => {
                        list.push(e.as_str().to_string());
                    },
                    None => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
            prepare_download(list, &mut paths.0, &mut paths.1, 3, false);
        },
        _ => {
            exit(0);
        }
    }
    println!("\nThank you for using SCDownloader
Please give this project a star <3
- GitHub : github.com/zeunig/scdownload
Got any issues? Join to my Discord server for support
- Discord : https://discord.gg/pJVxS6uRTK")
}