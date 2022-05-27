# API v3

This is version 3 of the Fates List API and backend upon which the Fates List frontend (sunbeam) and all other services sit upon. 

*Relies on baypaw (and flamepaw for push notifications) to work*

**This requires nightly rust in order to compile**

Internally known as Lightleap (also from Warrior Cats)

## TODOs (for my knowledge)

- Get Bot Filtered API (for dba etc)
- Finish refactor into ``ok`` and ``err`` in ``APIResponse``

## MacOS cross compile

Follow https://stackoverflow.com/questions/40424255/cross-compilation-to-x86-64-unknown-linux-gnu-fails-on-mac-osx

**Path update**

``PATH=/opt/homebrew/Cellar/x86_64-unknown-linux-gnu/7.2.0/bin:$PATH``
