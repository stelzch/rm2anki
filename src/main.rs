use std::{io::{Cursor, Read, Write}, path::{self, PathBuf}};


use zip::ZipArchive;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct IndexFile {
    fileType: String,
    formatVersion: i32,
    pageCount: u32,
    pages: Vec<String>
}

fn id_from_uuid(uuid: &str) -> i64 {
    let id = uuid::Uuid::parse_str(uuid).unwrap();

    id.as_u64_pair().1 as i64
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} <notebook zip> <target>", args[0]);
        std::process::exit(1);
    }

    let source = path::Path::new(&args[1]);
    let deck_name = source
        .file_stem().unwrap()
        .to_str().unwrap().to_owned();
    let target = &args[2];

    let file = std::fs::File::open(source).expect("Could not open notebook");
    let mut archive = ZipArchive::new(file).expect("Failed to read zip file");

    let content_filename = archive.file_names()
        .filter(|name| name.ends_with(".content"))
        .next()
        .expect("Could not find content file")
        .to_owned();

    let uuid = &content_filename[0..(content_filename.len() - ".content".len())];

    let mut json_contents = String::new();
    archive.by_name(&content_filename)
        .unwrap()
        .read_to_string(&mut json_contents)
        .unwrap();

    let data: IndexFile  = serde_json::from_str(&json_contents).unwrap();
    println!("Notebook {} has {} pages.", uuid, data.pageCount);


    let dir = tempdir().unwrap();
    data.pages.clone().into_iter().for_each(|page| {
        let path = format!("{uuid}/{page}.rm");
        println!("Reading page {}", path);

        let mut f = archive.by_name(&path).expect("Missing page data");
        
        let lines_data = lines_are_rusty::LinesData::parse(&mut f)
            .expect("Failed to parse line data");

        let lines_page = &lines_data.pages[0];

        let layer_colors = vec![lines_are_rusty::LayerColor::default(); lines_page.layers.len()];

        let filename = format!("{page}.svg");
        let out_path = dir.path().join(filename);
        let mut file = std::fs::File::create(out_path.clone()).unwrap();

        lines_are_rusty::render_svg(&mut file,
            &lines_page,
            false,
            &layer_colors,
            0.0001,
            None,
            false
        ).unwrap();
    });


    // Generate anki deck

let css = ""; //div.container { display: block; border: 1px solid red; width: 915px; height: 622px; overflow: hidden; } img { box-sizing: border-box; } .top >img { transform: translate(-17%, -15%); } .bottom > img { transform: translate(-17%, -52%); }";
    let model = || {
        genanki_rs::Model::new_with_options(
        8779108157104849531,
        "Remarkable Flashcard",
        vec![
            genanki_rs::Field::new("MediaFront"),
            genanki_rs::Field::new("MediaBack"),
        ],
        vec![genanki_rs::Template::new("Card 1")
        .qfmt("Question: {{MediaFront}}")
        .afmt("Answer: {{MediaBack}}")],
        Some(css),
        None, None, None, None)
    };

    let mut deck = genanki_rs::Deck::new(
        id_from_uuid(uuid),
        &deck_name,
        "Deck generated from remarkable notebook");


    let media_files: Vec<PathBuf> = data.pages.iter().map(|page: &String| {
        let filename = format!("{page}.svg");
        //let img = format!("<img src=\"{filename}\">");
        let img1 = format!("<svg viewBox=\"244.33 276 915.34 622.70\"><image width=\"1404\" height=\"1872\" href=\"{filename}\"></svg>");
        let img2 = format!("<svg viewBox=\"244.33 973.4636363636365 915.34 622.70\"><image width=\"1404\" height=\"1872\" href=\"{filename}\"></svg>");
        println!("{}", img1);
        let note = genanki_rs::Note::new(model(), vec![&img1, &img2]).unwrap();
        deck.add_note(note);

        let mut path = PathBuf::new();
        path.push(dir.path());
        path.push(&filename);

        if !path.exists() {
            println!("Warning, {:?} does not exist", filename);
        }

        path
    }).collect();

    let files = media_files.iter().map(|x: &PathBuf| x.to_str().unwrap()).collect();
    println!("Files to include: {:?}", files);


    let mut package = genanki_rs::Package::new(vec![deck],files).unwrap();
    package.write_to_file(target).unwrap();

    drop(dir);

}
