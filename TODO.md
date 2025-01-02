## General imporovements

- [ ] Fix file uploading!
- [ ] Maybe add some logic to make it wait for the socket to be opened?
  - [ ] also better error messages, that would be nice.
  - [ ] and it might be cool to give you the option to open the mpv instance by yourself
- [ ] Maybe write some integration tests with Nix's testing framework
- [ ] Make QR code move every once in a while to avoid burn-in
- [ ] Save playlist on exit and load it on start with the previous position. Maybe there's an option for this, otherwise: https://github.com/CogentRedTester/mpv-scripts/blob/master/save-playlist.lua
- [ ] Package a batch file that downloads the latest mpv release and creates a start.bat file on Windows

## Scope creep

- [ ] Integrate with aria2c for downloading torrents
- [ ] Integrate with https://github.com/9001/party-up
- [ ] Player controls (play/pause)
- [ ] Progress bar for currently playing file
- [ ] Announce video title when it starts playing
- [ ] Browse a local folder
- [ ] Announce queues?
- [ ] Make it possible to hide the QR code
- [ ] send email
