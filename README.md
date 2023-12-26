# SCDownload
![preview](https://cdn.discordapp.com/attachments/1152297609256521789/1152310916956835921/cmd_3kklI5awUW.gif) <br>
A simple-to-use command line tool for mass downloading tracks, playlists and albums, made with only 4 dependencies (id3,mp3cat,regex,reqwest)

## Usage
scdownload <track/album/playlist/artist> <id> 
### Example
scdownload playlist zeunig/sets/hardstyle
scdownload artist zeunig
scdownload track zeunig/garou-hardstyle
### Additional arguments
--temp-dir="path" : Changes the cache dir<br />
--download-dir="path" : Changes the download dir
--thread-count=number : Changes the amount of threads (only valid while downloading playlists)
--original-cover-size : Downloads the song's cover image in it's original size
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

# Views
<a href="https://hits.seeyoufarm.com"><img src="https://hits.seeyoufarm.com/api/count/incr/badge.svg?url=https%3A%2F%2Fgithub.com%2FZeunig%2FSCDownload%2F&count_bg=%2379C83D&title_bg=%23555555&icon=&icon_color=%23E7E7E7&title=hits&edge_flat=false"/></a>
