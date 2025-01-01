"use strict";

var playlistOl = document.getElementById("playlist");

function updatePlaylist() {
    fetch("/api/playlist")
        .then((resp) => resp.json())
        .then((playlist) => {
            playlistOl.textContent = "";

            playlist.forEach((item) => {
                var container,
                    li = document.createElement("li");

                if (item.filename.startsWith("http")) {
                    var a = document.createElement("a");
                    a.href = item.filename;
                    li.appendChild(a);
                    container = a;
                } else {
                    container = li;
                }

                container.textContent = item.filename;
                if (item.current) {
                    container.className = "currently-playing";
                }
                playlistOl.appendChild(li);
            });
        });
};

updatePlaylist();

setInterval(updatePlaylist, 10 * 1000);
