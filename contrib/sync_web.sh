#!/bin/zsh
echo This script requires modification before it can be run, it serves more as a reference than a ready-to-use solution.
exit 0

cd /home/user/my_flashcard_folder

if [[ ! -d notebooks ]]; then
    mkdir notebooks
fi

xochitl_flashcard_folder=/ # Can be directed for example at /Uni to only pick up notebooks that are inside that hierarchy.
files=$(rmapi find $xochitl_flashcard_folder Flashcards | grep -F '[f]' | cut -d' ' -f2- > index.txt)

IFS=$'\n'
cd notebooks
for file in $(<../index.txt); do
    name=$(echo $file | awk -F/ '{print $(NF-1)}') # Use the folder the notebook is contained in as deck name.
    rmapi get $file 
    mv Flashcards.zip $name.zip
done
cd ..
./AnkiSync --name-from-filename flashcards.apkg notebooks/*.zip
curl -X POST -d '{"action": "importPackage", "version": 6, "params": {"path": "'$(pwd)'/flashcards.apkg"}}' http://localhost:8765/
