use std::{io::{Cursor, Read}, path::{self, Path}};


use zip::ZipArchive;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct IndexFile {
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
    let target = &args[2];
    let target_p = path::Path::new(&args[2]);
    let deck_name = target_p
        .file_stem().unwrap()
        .to_str().unwrap().to_owned();

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


    // Generate anki deck
    let model = || {
        genanki_rs::Model::new(
        8779108157104849531,
        "Remarkable Flashcard",
        vec![
            genanki_rs::Field::new("MediaFront"),
            genanki_rs::Field::new("MediaBack"),
            genanki_rs::Field::new("DummyImage"), // This field is needed to make sure Anki picks
                                                  // up the image files. It does not recognize them
                                                  // based on the svg tags alone.
        ],
        vec![genanki_rs::Template::new("Card 1")
        .qfmt("Question: {{MediaFront}}")
        .afmt("Answer: {{MediaBack}}")])
    };

    let mut deck = genanki_rs::Deck::new(
        id_from_uuid(uuid),
        &deck_name,
        "Deck generated from remarkable notebook");

    let media_files: Vec<genanki_rs::MediaFile> = data.pages.clone().into_iter().map(|page| {
        // Add note
        let filename = format!("{page}.svg");
        //let img = format!("<img src=\"{filename}\">");
        let img1 = format!("<svg viewBox=\"244.33 276 915.34 622.70\"><image width=\"1404\" height=\"1872\" href=\"{filename}\"></svg>");
        let img2 = format!("<svg viewBox=\"244.33 973.4636363636365 915.34 622.70\"><image width=\"1404\" height=\"1872\" href=\"{filename}\"></svg>");
        let img3 = format!("<img src=\"{filename}\">");
        println!("{}", img1);
        let note = genanki_rs::Note::new(model(), vec![&img1, &img2, &img3]).unwrap();
        deck.add_note(note);


        // Render media file
        let path = format!("{uuid}/{page}.rm");
        println!("Reading page {}", path);

        let mut f = archive.by_name(&path).expect("Missing page data");
        
        let lines_data = lines_are_rusty::LinesData::parse(&mut f)
            .expect("Failed to parse line data");

        let lines_page = &lines_data.pages[0];

        let layer_colors = vec![lines_are_rusty::LayerColor::default(); lines_page.layers.len()];

        let filename = format!("{page}.svg");

        let mut svg_contents: Vec<u8> = vec![];
        let mut c = Cursor::new(&mut svg_contents);
        lines_are_rusty::render_svg(&mut c,
            &lines_page,
            false,
            &layer_colors,
            0.0001,
            None,
            false
        ).unwrap();

        genanki_rs::MediaFile::Bytes(svg_contents, filename)
    }).collect();


    let mut package = genanki_rs::Package::new_from_memory(vec![deck], media_files).unwrap();
    package.write_to_file(target).unwrap();
}
