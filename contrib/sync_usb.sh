#!/bin/zsh
echo This script requires modification before it can be run, it serves more as a reference than a ready-to-use solution.
exit 0

cd /home/user/my_flashcard_folder

if [[ ! -d notebooks ]]; then
    mkdir notebooks
fi

# UUIDs of notebooks (space separated)
uuids=()

# Corresponding deck names
names=()

# You need to either modify the below source or configure your ssh so you can address
# your remarkable with only the host name. Here is my .ssh/config:
#
# Host remarkable
#    Hostname 10.11.99.1
#    User root
#    Port 22
#    IdentityFile ~/.ssh/id_rsa
remarkable_src=remarkable:/home/root/.local/share/remarkable/xochitl
if [[ ! -d notebooks_raw ]]; then
    mkdir notebooks_raw
fi
cd notebooks_raw
for i in {1..$#uuids}; do
    uuid=${uuids[$i]}
    name=${names[$i]}
    target=../notebooks/$name.zip
    echo Loading $uuid to $target
    rsync -auv $remarkable_src/$uuid\* .
    zip -r $target $uuid* 
done
cd ..
./AnkiSync --name-from-filename flashcards.apkg notebooks/*.zip
curl -X POST -d '{"action": "importPackage", "version": 6, "params": {"path": "'$(pwd)'/flashcards.apkg"}}' http://localhost:8765/
