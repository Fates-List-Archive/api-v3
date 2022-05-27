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

**Use https://github.com/MaterializeInc/homebrew-crosstools for cross compiling as it is newer**

**Path update**

``PATH=/opt/homebrew/Cellar/x86_64-unknown-linux-gnu/7.2.0/bin:$PATH``

**Not always needed, try running ``make cross`` before doing the below**

Symlink ``gcc`` if needed by ring at ``/opt/homebrew/Cellar/x86_64-unknown-linux-gnu/7.2.0/bin`` based on the error you get

Replace 7.2.0 with whatever gcc version you need

``make push`` to push newly built lib. Mofidy according to your ssh ip
