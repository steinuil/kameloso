"use strict";

var nowPlayingEl = document.getElementById("np");
var queueEl = document.getElementById("queue");
var playedEl = document.getElementById("played");

/**
 * @typedef {object} PlaylistEntry
 * @property {number} id
 * @property {string} filename
 * @property {string=} title
 * @property {boolean} current
 * @property {boolean} playing
 */

/**
 * @param {PlaylistEntry} entry
 * @param {string} type
 */
function renderPlaylistEntry(entry, type) {
  var container = document.createElement(type);
  /** @type {HTMLElement} */
  var textContainer;

  if (entry.filename.startsWith("http")) {
    var a = document.createElement("a");
    a.href = entry.filename;
    container.appendChild(a);
    textContainer = a;
  } else {
    textContainer = container;
  }

  textContainer.textContent = entry.title || entry.filename;

  return container;
}

/**
 * @param {PlaylistEntry} entry
 * @returns {HTMLElement[]}
 */
function renderNowPlaying(entry) {
  if (entry.filename.startsWith("http")) {
    var a = document.createElement("a");
    a.href = entry.filename;
    a.textContent = entry.filename;

    if (!entry.title) {
      return [a];
    }

    var span = document.createElement("span");
    span.textContent = entry.title;

    return [a, span];
  }

  var span = document.createElement("span");
  span.textContent = entry.title || entry.filename;

  return [span];
}

/**
 * @param {PlaylistEntry[]} entries
 */
function render(entries) {
  var currentIndex = entries.findIndex((entry) => entry.playing);

  if (currentIndex == -1) {
    playedEl.replaceChildren();
    queueEl.replaceChildren();
    return;
  }

  var nowPlaying = entries[currentIndex];
  var played = entries.slice(0, currentIndex);
  var queue = entries.slice(currentIndex + 1);

  var playedLinks = played
    .reverse()
    .map((entry) => renderPlaylistEntry(entry, "li"));
  var queueLinks = queue.map((entry) => renderPlaylistEntry(entry, "li"));

  nowPlayingEl.replaceChildren(...renderNowPlaying(nowPlaying));

  playedEl.replaceChildren(...playedLinks);
  queueEl.replaceChildren(...queueLinks);
}

function updatePlaylist() {
  fetch("/api/playlist")
    .then((resp) => resp.json())
    .then((playlist) => render(playlist));
}

updatePlaylist();

setInterval(updatePlaylist, 10 * 1000);
