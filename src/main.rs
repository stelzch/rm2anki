use std::{io::{Cursor, Read, Write}, path::{self, PathBuf}, fs::File};


use clap::Parser;
use zip::{ZipArchive, read::ZipFile};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NotebookIndex {
    pages: Vec<String>
}

#[derive(Debug, Deserialize)]
struct NotebookMetadata {
    visibleName: String
}

fn id_from_uuid(uuid: &str) -> i64 {
    let id = uuid::Uuid::parse_str(uuid).unwrap();

    id.as_u64_pair().1 as i64
}

fn remarkable_model() -> genanki_rs::Model {
    let css = "
div.container {
    all: initial;
    display: block;
    overflow: hidden;
    width: 100%;
    aspect-ratio: 915 / 622;
}
.chrome img {
    /* Overwrite AnkiDroid stylesheet */
    max-width: none;
    max-height: none;
}

img {
    overflow: hidden;
    position: relative;
    width: 153%;
    height: 303%;
    left: -26%;

    /* Overwrite default Anki values */
    max-width: none;
    max-height: none;
}

.front {
    top: -44%;
}

.back {
    top: -157%;
}
";
    genanki_rs::Model::new_with_options(
    8779108157104849532,
    "Remarkable Flashcard v2",
    vec![
        genanki_rs::Field::new("MediaFront"),
        genanki_rs::Field::new("MediaBack")
    ],
    vec![genanki_rs::Template::new("Card 1")
    .qfmt("{{MediaFront}}")
    .afmt("{{MediaBack}}")],
    Some(css),
    None, None, None, None
    )
}

fn render_media_file(f: &mut ZipFile) -> Result<Vec<u8>, String> {
        let lines_data = lines_are_rusty::LinesData::parse(f)
            .map_err(|_| "Failed to parse line data")?;

        let lines_page = &lines_data.pages[0];

        let layer_colors = vec![lines_are_rusty::LayerColor::default(); lines_page.layers.len()];


        let mut svg_contents: Vec<u8> = vec![];
        let mut c = Cursor::new(&mut svg_contents);
        lines_are_rusty::render_svg(&mut c,
            &lines_page,
            false,
            &layer_colors,
            0.0001,
            None,
            false
        ).map_err(|_| format!("Rendering failed"))?;

        Ok(svg_contents)
}

struct ConvertedDeck {
    deck: genanki_rs::Deck,
    media_files: Vec<genanki_rs::MediaFile>
}

fn read_metadata(zip: &mut ZipArchive<File>, uuid: &str) -> Result<NotebookMetadata, String> {

    let filename = format!("{uuid}.metadata");
    let mut file = zip.by_name(&filename)
        .map_err(|_| format!("Could not find metadata file for notebook {uuid}"))?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .map_err(|_| format!("Could not read metadata file for notebook {uuid}"))?;

    serde_json::from_str(&file_contents)
        .map_err(|_| format!("Could not parse metadata file for notebook {uuid}"))
}

fn read_index(zip: &mut ZipArchive<File>, uuid: &str) -> Result<NotebookIndex, String> {
    let filename = format!("{uuid}.content");
    let mut file = zip.by_name(&filename)
        .map_err(|_| format!("Could not find contents file for notebook {uuid}"))?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .map_err(|_| format!("Could not read contents file for notebook {uuid}"))?;

    serde_json::from_str(&file_contents)
        .map_err(|_| format!("Could not parse contents file for notebook {uuid}"))
}

fn convert_to_anki_deck(source: &PathBuf, name_from_filename: bool, anki_media_dir: Option<&path::Path>) -> Result<ConvertedDeck, String> {
    let file = std::fs::File::open(source)
        .map_err(|_| "Could not open notebook zip file")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Failed to read zip file")?;


    // Open file that lists page contents
    let content_filename = archive.file_names()
        .filter(|name| name.ends_with(".content"))
        .next()
        .expect("Could not find content file")
        .to_owned();

    let uuid = &content_filename[0..(content_filename.len() - ".content".len())];
    let metadata = read_metadata(&mut archive, uuid)?;
    let index = read_index(&mut archive, uuid)?;

    let deck_name = if name_from_filename {
        source.file_stem().ok_or("Can not extract deck name from filename")?
            .to_str().ok_or("Invalid filename, can not use as deck name")?
    } else {
        &metadata.visibleName
    };

    println!("Processing deck {}", deck_name);

    let mut deck = genanki_rs::Deck::new(
        id_from_uuid(uuid),
        deck_name,
        "Deck generated from remarkable notebook");

    let media_files: Vec<genanki_rs::MediaFile> = index.pages.clone().into_iter().map(|page| {
        // Add note
        let filename = format!("{page}.svg");
        let note = genanki_rs::Note::new(
            remarkable_model(),
            vec![&field_template_front(&filename),
                 &field_template_back(&filename)])
            .map_err(|_| format!("Could not create note from page {page}"))?;
        deck.add_note(note);

        // Render media file
        let path = format!("{uuid}/{page}.rm");
        let mut f = archive.by_name(&path).expect("Missing page data");
        let filename = format!("{page}.svg");
        let svg_contents = render_media_file(&mut f)?;

        // Write to Anki collection
        if let Some(path) = anki_media_dir {
            let target = path.join(&filename);
            let error_msg = format!("Could not write to Anki media file {}", filename);
            println!("Writing to {:?}", target);

            let mut media_file = File::create(target)
                .map_err(|_| &error_msg)?;

            media_file.write_all(&svg_contents)
                .map_err(|_| &error_msg)?;
        }
        

        Ok(genanki_rs::MediaFile::Bytes(svg_contents, filename))
    }).collect::<Result<Vec<genanki_rs::MediaFile>, String>>()?;


    Ok(ConvertedDeck {
        deck, media_files
    })
}

fn field_template_front(filename: &str) -> String {
    format!("<div class=\"container\"><img class=\"front\" src=\"{filename}\"></div>")
}
fn field_template_back(filename: &str) -> String {
    format!("<div class=\"container\"><img class=\"back\" src=\"{filename}\"></div>")
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help="Path to Anki collection.media directory")]
    anki_media_dir: Option<PathBuf>,

    #[arg(long, short, action, help="Use zipfile basename as deck name. Default is to use notebook name.")]
    name_from_filename: bool,


    #[arg(help="Path to the destination .apkg")]
    output: String,

    #[arg(help="Path to a notebook in rmapi zip file format")]
    notebooks: Vec<PathBuf>,

}


fn decks_to_package(v: Vec<ConvertedDeck>) -> Result<genanki_rs::Package, String> {

    let (decks, media_files_nested): (Vec<_>, Vec<_>) = v.into_iter().map(|x|
        (x.deck, x.media_files)
    ).unzip();

    let media_files: Vec<genanki_rs::MediaFile> = media_files_nested.into_iter().flatten().collect();

    genanki_rs::Package::new_from_memory(decks, media_files)
        .map_err(|_| "Could not create Anki package".to_owned())
}

fn main() {
    let args = Args::parse();

    let anki_media_dir = match args.anki_media_dir.as_ref() {
        Some(v) => Some(v.as_path()),
        None => None
    };


    let converted_decks: Vec<ConvertedDeck> = args.notebooks.iter()
        .map(|p| convert_to_anki_deck(p, args.name_from_filename, anki_media_dir))
        .map(|x| {
            match x {
                Ok(d) => {
                    return Some(d);
                },
                Err(e) => {
                    eprintln!("{}", e);
                    return None;
                }
            }
        })
    .flatten()
    .collect();

    let deck_num = converted_decks.len();

    match decks_to_package(converted_decks) {
        Ok(mut package) => {
            let result = package.write_to_file(&args.output);

            if result.is_ok() {
                println!("Wrote {} decks to the package", deck_num);
            } else {
                eprintln!("Could not write package to file: {}", result.unwrap_err());
            }

        },
        Err(s) => {
            eprintln!("{}", s);
        }
    }
}
