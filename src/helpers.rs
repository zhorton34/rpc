use serde_json;
use lopdf::Document;
use std::io::Cursor;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use crate::errors::{PdfExtractError, ParseSouthLawPropertiesError};


#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct Property {
    pub id: String,
    pub state: String,
    pub county: String,
    pub street: String,
    pub city: String,
    pub zip: String,
    pub sale_date: String,
    pub sale_time: String,
    pub continued_date_time: String,
    pub opening_bid: String,
    pub sale_location_city: String,
    pub firm_file_number: String,
}

impl Property {
    // Constructor to create a new empty Property
    pub fn new() -> Self {
        Property {
            id: String::new(),
            county: String::new(),
            street: String::new(),
            city: String::new(),
            state: String::new(),
            zip: String::new(),
            sale_date: String::new(),
            sale_time: String::new(),
            continued_date_time: String::new(),
            opening_bid: String::new(),
            sale_location_city: String::new(),
            firm_file_number: String::new(),
        }
    }

    // Setters
    pub fn set(&mut self, idx: String, value: String) {
        if idx == "0" {
            self.county = value;
        } else if idx == "1" {
            self.street = value;
        } else if idx == "2" {
            self.city = value;
        } else if idx == "3" {
            self.zip = value;
        } else if idx == "4" {
            self.sale_date = value;
        } else if idx == "5" {
            self.sale_time = value;
        } else if idx == "6" {
            self.continued_date_time = value;
        } else if idx == "7" {
            self.opening_bid = value;
        } else if idx == "8" {
            self.sale_location_city = value;
        } else if idx == "9" {
            self.firm_file_number = value;
        } else if idx == "10" {
            self.state = value;
        } else if idx == "11" {
            self.id = value;
        }
    }
}

pub fn is_street_address(segment: &str) -> bool {
    let parts: Vec<&str> = segment.split_whitespace().collect();

    // Check if the first part is a number and there are additional parts for the street name
    parts.first().map_or(false, |first_part| first_part.chars().all(char::is_numeric)) &&
    parts.len() > 1
}

pub async fn extract_pdf_text(url: &str) -> Result<String, PdfExtractError> {
    let client = reqwest::Client::new();
    let res = client.get(url).send().await?;
    let bytes = res.bytes().await?;
    let mut cursor = Cursor::new(bytes);
    let doc = Document::load_from(&mut cursor).unwrap();
    println!("PDF has {} pages.", doc.get_pages().len());
    let total_pages = doc.get_pages().len() as u32;
    let page_numbers: Vec<u32> = (1..=total_pages).collect();
    let text = doc.extract_text(&page_numbers)?;

    Ok(text)
} 

pub fn extract_lines_from_pdf(text: String) -> Result<Vec<String>, ParseSouthLawPropertiesError> {
    if text.is_empty() {
        return Err(ParseSouthLawPropertiesError::EmptyInputError);
    }

    let excluded_phrases = [
        "Foreclosure Sales", 
        "Information Reported as of:",
        "Property Address", 
        "Property City",
        "Sale Date",
        "Sale Time",
        "Continued Date/Time",
        "Opening Bid",
        "Sale Location(City)",
        "Civil Case No.",
        "Firm File#",
        "Property Zip",
        "13160 Foster, Ste. 100",
    ];

    let numeric_exclusions = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

    let lines = text
        .split("\n")
        .filter(|line| {
            !excluded_phrases.iter().any(|phrase| line.contains(phrase))
            && !numeric_exclusions.contains(line)
        })
        .map(|line| line.trim())
        .collect::<Vec<&str>>();

    if lines.is_empty() { return Err(ParseSouthLawPropertiesError::NoValidDataError) };

    lines.iter().for_each(|line| println!("{}", line));

    Ok(lines.iter().map(|line| line.to_string()).collect::<Vec<String>>())
}

pub async fn parse_southlaw_properties(text: String) -> Result<Vec<Property>, ParseSouthLawPropertiesError> {
    let state = if text.contains("Foreclosure Sales Report: Missouri") {
        String::from("MO")
    } else if text.contains("Foreclosure Sales Report: Kansas") {
        String::from("KS")
    } else if text.contains("Foreclosure Sales Report: Iowa") {
        String::from("IA")
    } else if text.contains("Foreclosure Sales Report: Nebraska") {
        String::from("NE")
    } else {
        String::from("Unknown")
    };

    println!("state: {:?}", state);
    let data = extract_lines_from_pdf(text).unwrap().join("|");
    
    let mut entries = data.split('|').collect::<Vec<_>>().to_vec();
    entries.pop();
    
    let mut idx: usize = 0;
    let mut county: String = "".to_string();
    let mut properties: Vec<Property> = Vec::new();
    for value in entries.iter() {
        if Some(value).is_none() {
            continue;
        }
        
        if idx == 0 {
            properties.push(Property::new());
            println!("properties: {:?}", properties);
            properties.last_mut().unwrap().set("10".to_string(), state.to_string());
            properties.last_mut().unwrap().set(idx.to_string(), county.to_string());
            if is_street_address(value) {
                // Skip the county and move to the next element
                println!("{:?}: {:?}", idx, county);
                properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
                
                idx += 1; // Fast forward by one increment
                println!("{:?}: {:?}", idx, value);
                properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
            } else {
                // This is a county
                county = value.to_string();
                println!("{:?}: {:?}", idx, county);
                properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
            }
        } else {
            println!("{:?}: {:?}", idx, value);
            properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
        }

        // Increment idx and reset if it reaches 10
        idx = (idx + 1) % 10;
    }

    let properties_with_id = properties.to_owned().into_iter().map(|property| {
        let concatenated = format!("{}{}{}{}{}{}{}{}{}{}{}", 
        property.county, property.street, property.city, property.state, property.zip, 
        property.sale_date, property.sale_time, property.continued_date_time, 
        property.opening_bid, property.sale_location_city, property.firm_file_number);

        let mut hasher = Sha256::new();
        hasher.update(concatenated);
        let result = hasher.finalize();

        let mut new_property = Property::new();
        new_property.set("0".to_string(), property.county);
        new_property.set("1".to_string(), property.street);
        new_property.set("2".to_string(), property.city);
        new_property.set("3".to_string(), property.zip);
        new_property.set("4".to_string(), property.sale_date);
        new_property.set("5".to_string(), property.sale_time);
        new_property.set("6".to_string(), property.continued_date_time);
        new_property.set("7".to_string(), property.opening_bid);
        new_property.set("8".to_string(), property.sale_location_city);
        new_property.set("9".to_string(), property.firm_file_number);
        new_property.set("10".to_string(), property.state);
        new_property.set("11".to_string(), format!("{:x}", result));
        
        new_property
    }).collect::<Vec<Property>>();

    println!("Properties: {:?}", properties_with_id);
    Ok(properties_with_id)
}