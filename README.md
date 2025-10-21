# SCDownload
![preview](https://cdn.discordapp.com/attachments/1152297609256521789/1152310916956835921/cmd_3kklI5awUW.gif) <br>
A simple-to-use command line tool for mass downloading tracks, playlists and albums, made with only 4 dependencies (id3,ffmpeg,regex,reqwest)

## Usage
scdownload <track/album/playlist/artist/liked> <url> 
### Example
scdownload playlist zeunig/sets/hardstyle<br />
scdownload artist zeunig<br />
scdownload track zeunig/garou-hardstyle<br />
scdownload liked zeunig
### Additional arguments
--temp-dir="path" : Changes the cache dir<br />
--download-dir="path" : Changes the download dir<br />
--thread-count=number : Changes the amount of threads (only valid while downloading playlists)<br />
--original-cover-size false|true : Downloads the song's cover image in it's original size<br />
--disable-cache false|true : Force redownload<br />
## Features
- Caching
- ID3
- Artist, cover, title parsing
- Track/album/playlist support
- Easy to use
- 4 dependencies only
- Multithreading
- Made in Rust

## Contact
[Discord server](https://discord.gg/pJVxS6uRTK)<br />
[Website](https://zeunig.hu)<br/>
[E-mail (not very responsive there)](mailto:business@mail.zeunig.hu)
