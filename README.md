ireul
=====

[![Build Status](https://travis-ci.org/infinityb/ireul.svg?branch=master)](https://travis-ci.org/infinityb/ireul)

An Ogg-specific Internet Radio backend.

Running
=======

    % cp example-config.toml config.toml
    % vim config.toml
    % cargo run --bin ireul-core ./config.toml

then, in another terminal, you may pass ogg files to the server:

    % cargo run --bin ireul-client enqueue ./howbigisthis.ogg

and the song will be added to the playlist.
