use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::fs;
use kuchikiki::traits::*;
use kuchikiki::parse_html;

struct Power {
    name: String,
    usage: String,
    text: String,
}

// Function to extract power names from XML
fn extract_power_names(xml_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(xml_path)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut power_names = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"Power" => {
                if let Some(attr) = e.attributes().find(|a| a.as_ref().unwrap().key == b"name") {
                    power_names.push(String::from_utf8(attr?.value.into_owned())?);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Box::new(e)),
            _ => (),
        }
        buf.clear();
    }

    Ok(power_names)
}

// Function to retrieve text fields from the database
fn get_power_texts_from_db(db_path: &str, power_names: &[String]) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    let mut texts = Vec::new();

    for name in power_names {
        let mut stmt = conn.prepare("SELECT Name, Usage, Txt FROM powers WHERE Name = ?1")?;
        let power_result: Option<Power> = stmt.query_row(params![name], |row| {
            Ok(Power {
                name: row.get(0)?,
                usage: row.get(1)?,
                text: row.get(2)?,
            })
        }).optional()?;
        if power_result.is_none() {
            eprintln!("Power {} not found in the database", name);
            continue;
        }
        let power = power_result.unwrap();

        let dom = parse_html().one(power.text);
        let detail_element = dom.select_first("#detail").unwrap();
        (|| {
            let mut attributes = detail_element.attributes.borrow_mut();
            if let Some(id) = attributes.get_mut("id") {
                *id = power.name.clone();
            }
            let classes = format!("Power {}", power.usage);
            if let Some(class) = attributes.get_mut("class") {
                *class = classes;
            }
            else
            {
                attributes.insert("class".to_string(), classes);
            }
        })();

        let mut output_html = Vec::new();
        let serialiazation_result = detail_element.as_node().serialize(&mut output_html);

        if serialiazation_result.is_err() {
            eprintln!("Failed to serialize power {}", power.name);
            continue;
        }
        // Print or save the serialized node content
        let element_as_text = format!("{}", String::from_utf8(output_html.clone()).unwrap());
        texts.push(element_as_text);
    }

    Ok(texts)
}

// Function to generate HTML file
fn generate_html(output_path: &str, texts: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;
    file.write_all(b"<html>
    <head>
        <title>Power Texts</title>
        <style>
        @media print {
          .grid {
            display: block !important;
            column-count: 3;
            column-gap: 20px; /* Adjust the gap between columns as needed */
            print-color-adjust: exact;
          }

          .Power {
            margin-bottom: 20px;
            page-break-inside: avoid;
            break-inside: avoid;
          }
        }
        .grid {
          display: grid;
          grid-template-columns: repeat(4, 1fr); /* Set the number of columns here */
          grid-gap: 20px;
        }
        .Power {
          border: 1px solid #ddd;
          display: flex;
          flex-direction: column;
        }
        .Power h1 {
          font-size: 1.5em;
          margin: 0;
          padding: 3px 10px;
        }
        .Power h1 span {
          font-size: 0.4em;
          font-weight: normal;
          display: block;
          opacity: 0.8;
        }
        .Power p {
          padding: 0px 10px;
          margin: 0;
          margin-bottom: 10px;
          padding-top: 3px;
          padding-bottom: 3px;
        }
        .Power .publishedIn {
          justify-self: flex-end;
        }
        .Power .flavor {
          background: rgb(215,213,198);
          background: linear-gradient(90deg, rgba(215,213,198,1) 0%, rgba(215,213,198,0) 100%);
        }
        .At-Will h1 {
          background-color: rgb(108, 150, 109);
          color: white;
        }
        .Encounter h1 {
          background-color: rgb(137, 35, 54);
          color: white;
        }
        .Daily h1 {
          background-color: rgb(77, 77, 79);
          color: white;
        }
        </style>
      </head>
    <body>
        <div class='grid'>")?;

    for text in texts {
        file.write_all(b"<div class='power'>")?;
        file.write_all(text.as_bytes())?;
        file.write_all(b"</div>")?;
    }

    file.write_all(b"</div></body></html>")?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml_dir = "./data/characters";
    let db_path = "./data/powers.db";
    let output_path = "./data/printables";
    let entries = fs::read_dir(xml_dir)?;
    for entry in entries {
        let entry = entry?;
        let xml_path: PathBuf = entry.path();
        if xml_path.is_file() && xml_path.extension().unwrap_or_default() == "dnd4e" {
            let power_names = extract_power_names(xml_path.to_str().unwrap())?;
            print!("{:?}", power_names);
            let texts = get_power_texts_from_db(db_path, &power_names)?;
            let mut output_html_path: PathBuf = [output_path, &xml_path.file_stem().unwrap().to_string_lossy()].iter().collect();
            output_html_path.set_extension("html");
            generate_html(&output_html_path.to_string_lossy(), &texts)?;
            print!("Generated {}\n", output_html_path.to_string_lossy());
        }
    }

    Ok(())
}
