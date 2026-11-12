//! Headless Hug's Bouwboek: the engineer's report for the permit, drafted here
//! so ENS only checks and signs it. The prose is the Dutch template
//! `docs/bouwboek/headless-hug.md`; every `{{field}}` in it is filled from the
//! fabric JSON and the parts list of the same build as the rest of the package.
//! Its figures are the package's `images/` (`wgpu::figure`).

use crate::connector::parts::{parts_list, placements, Part};
use crate::fabric::Fabric;
use crate::wgpu::figure::PartFigure;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub const TEMPLATE: &str = "docs/bouwboek/headless-hug.md";

/// The template with its fields filled and its opening comment, the notes for
/// whoever edits it, left out. Panics on a field it doesn't know, so the
/// template and this stay in step.
pub fn document(fabric: &Fabric, json: &Value, figures: &[PartFigure], template: &str) -> String {
    let body = template.split_once("-->").map_or(template, |(_, body)| body);
    let mut doc = body.trim_start().to_string();
    for (field, value) in fields(fabric, json, figures) {
        doc = doc.replace(&format!("{{{{{field}}}}}"), &value);
    }
    if let Some(start) = doc.find("{{") {
        let end = doc[start..].find("}}").map_or(doc.len(), |end| start + end + 2);
        panic!("the Bouwboek doesn't fill {}", &doc[start..end]);
    }
    doc
}

/// Dutch: a decimal comma.
fn nl(x: f64, decimals: usize) -> String {
    format!("{x:.decimals$}").replace('.', ",")
}

fn array<'a>(json: &'a Value, key: &str) -> &'a [Value] {
    json[key].as_array().map(Vec::as_slice).unwrap_or_else(|| panic!("JSON has no {key}"))
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or_else(|| panic!("not a number: {value}"))
}

fn bounds(values: impl Iterator<Item = f64>) -> (String, String) {
    let (low, high) = values.fold((f64::MAX, f64::MIN), |(low, high), x| (low.min(x), high.max(x)));
    (nl(low, 2), nl(high, 2))
}

/// For "tussen … en …".
fn range(values: impl Iterator<Item = f64>) -> String {
    let (low, high) = bounds(values);
    format!("{low} m en {high} m")
}

/// For a table cell.
fn span(values: impl Iterator<Item = f64>) -> String {
    match bounds(values) {
        (low, high) if low == high => low,
        (low, high) => format!("{low} – {high}"),
    }
}

fn kn(newtons: f64) -> String {
    nl(newtons / 1000.0, 2)
}

fn fields(fabric: &Fabric, json: &Value, figures: &[PartFigure]) -> Vec<(&'static str, String)> {
    let struts = array(json, "struts");
    let tension = array(json, "tension");
    let guy_lines = array(json, "guy_lines");
    let anchors: BTreeSet<&str> = guy_lines.iter().filter_map(|guy| guy["anchor"].as_str()).collect();
    let positions: Vec<[f64; 3]> = array(json, "joints")
        .iter()
        .filter(|joint| !anchors.contains(joint["name"].as_str().unwrap_or_default()))
        .map(|joint| [0, 1, 2].map(|axis| number(&joint["position"][axis])))
        .collect();
    let extent = |axis: usize| {
        let (low, high) = positions.iter().fold((f64::MAX, f64::MIN), |(low, high), p| (low.min(p[axis]), high.max(p[axis])));
        nl(high - low, 2)
    };
    let cables = || tension.iter().filter(|t| t["kind"] == "cable");
    let largest = |members: &mut dyn Iterator<Item = &Value>| {
        members.filter_map(|member| member["force"].as_f64()).map(f64::abs).fold(0.0, f64::max)
    };
    vec![
        ("datum", chrono::Local::now().format("%-d-%-m-%Y").to_string()),
        ("lengte", extent(0)),
        ("breedte", extent(1)),
        ("hoogte", extent(2)),
        ("aantal_staven", struts.len().to_string()),
        ("aantal_kabels", cables().count().to_string()),
        ("aantal_gaffelstangen", tension.iter().filter(|t| t["kind"] != "cable").count().to_string()),
        ("aantal_knoopplaten", array(json, "plates").len().to_string()),
        ("aantal_scharnieren", array(json, "hinges").len().to_string()),
        ("aantal_voeten", array(json, "feet").len().to_string()),
        ("aantal_tuidraden", guy_lines.len().to_string()),
        ("staaflengtes", range(struts.iter().map(|s| number(&s["rest_length"])))),
        ("kabellengtes", range(cables().map(|c| number(&c["rest_pin_to_pin"])))),
        ("max_staafkracht", kn(largest(&mut struts.iter()))),
        ("max_kabelkracht", kn(largest(&mut cables()))),
        ("tabel_onderdelen", parts_table(fabric, json)),
        ("tabel_gewicht", mass_table(json)),
        ("tabel_staven", strut_table(struts)),
        ("tabel_trekelementen", tension_table(tension)),
        ("tabel_voeten", feet_table(json)),
        ("tabel_tuidraden", guy_table(guy_lines)),
        ("figuren_onderdelen", part_figures(figures)),
    ]
}

/// A part's Dutch name and material, as in Open Claw's report where it had one.
fn onderdeel(part: Part) -> Option<(&'static str, &'static str)> {
    Some(match part {
        Part::OuterTube(_) => ("Aluminium buis 40x3 mm, buitenbuis", "EN AW-6060 T66"),
        Part::InnerTube(_) => ("Aluminium buis 33x3 mm, binnenbuis", "EN AW-6060 T66"),
        Part::Ring => ("Ring met nok en dwarsbuis", "staal, verzinkt"),
        Part::CustomDisc(_) => ("Schijf in kopverbinding, op maat", "staal, verzinkt"),
        Part::Washer => ("Afstandhouder", "POM"),
        Part::Clevis => ("Gaffelterminal voor kabel 6 mm", "AISI 316"),
        Part::RodFork => ("Gaffel aan gaffelstang", "staal, verzinkt"),
        Part::Hinge => ("Scharnier GN 751 / GN 752 M12", "staal"),
        Part::FootRodEnd => ("Stangkop INA GIKR12-PW, voet", "staal"),
        Part::NodePlate(_) => ("Knoopplaat", "staal, verzinkt"),
        Part::ForkNuts | Part::HingeEye => return None,
    })
}

/// The views are figures 1 to 4, written in the template; the parts follow.
const FIRST_PART_FIGURE: usize = 5;

fn part_figures(figures: &[PartFigure]) -> String {
    let mut text = String::new();
    for (index, figure) in figures.iter().enumerate() {
        let Some((name, _)) = onderdeel(figure.part) else { continue };
        let length = match figure.part {
            Part::OuterTube(mm) | Part::InnerTube(mm) => format!(", lengte {mm} mm"),
            _ => String::new(),
        };
        let count = match figure.count {
            1 => "1 stuk".to_string(),
            n => format!("{n} stuks"),
        };
        let caption = format!("Figuur {} {name}{length} ({}), {count}", FIRST_PART_FIGURE + index, figure.letter);
        writeln!(text, "![{caption}](images/{})\n\n*{caption}*\n", figure.file).unwrap();
    }
    text
}

fn parts_table(fabric: &Fabric, json: &Value) -> String {
    let mut rows: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (part, count) in parts_list(&placements(fabric)) {
        if let Some(row) = onderdeel(part) {
            *rows.entry(row).or_default() += count;
        }
    }
    let struts = array(json, "struts");
    let tension = array(json, "tension");
    let ends = || struts.iter().flat_map(|strut| array(strut, "ends"));
    let cables: Vec<&Value> = tension.iter().filter(|t| t["kind"] == "cable").collect();
    let cable_length: f64 = cables.iter().map(|c| number(&c["rest_pin_to_pin"])).sum();
    let mut table = String::from("| Aantal | Omschrijving | Materiaal |\n|---:|---|---|\n");
    for ((name, material), count) in rows {
        writeln!(table, "| {count} | {name} | {material} |").unwrap();
    }
    let stock = [
        (cables.len(), format!("Staalkabel 6 mm 7x19, verzinkt, samen {} m", nl(cable_length, 1)), "EN 12385-4"),
        (tension.len() - cables.len(), "Draadstang M8 in gaffelstang".to_string(), "staal, verzinkt"),
        (ends().count(), "Kop: stalen buis 40x3 mm met kopplaat".to_string(), "S235"),
        (ends().count(), "Bout M12".to_string(), "8.8"),
        (ends().filter(|end| end["closed_by"] == "washer and lock nut").count(), "Borgmoer M12 en sluitring".to_string(), "8"),
        (array(json, "guy_lines").len(), "Tuidraad, staalkabel 6 mm, naar grondanker".to_string(), "EN 12385-4"),
    ];
    for (count, name, material) in stock {
        writeln!(table, "| {count} | {name} | {material} |").unwrap();
    }
    table
}

fn mass_table(json: &Value) -> String {
    let struts = array(json, "struts");
    let tension = array(json, "tension");
    let per_metre: BTreeMap<&str, f64> = array(json, "tension_stock")
        .iter()
        .map(|stock| (stock["name"].as_str().unwrap_or_default(), number(&stock["mass_per_metre"])))
        .collect();
    let sum = |values: &mut dyn Iterator<Item = f64>| values.sum::<f64>();
    let rows = [
        ("Aluminium buizen", sum(&mut struts.iter().map(|s| number(&s["tube_mass"])))),
        (
            "Koppen aan de staven: kop, bout, moer en ringen",
            sum(&mut struts.iter().flat_map(|s| array(s, "ends")).map(|end| number(&end["hardware_mass"]))),
        ),
        (
            "Aansluitingen: terminal of gaffel, ring en afstandhouder",
            sum(&mut tension.iter().flat_map(|t| array(t, "ends")).map(|end| number(&end["hardware_mass"]))),
        ),
        (
            "Kabels en gaffelstangen",
            sum(&mut tension.iter().map(|t| {
                number(&t["rest_pin_to_pin"]) * per_metre[t["stock"].as_str().unwrap_or_default()]
            })),
        ),
        ("Knoopplaten", sum(&mut array(json, "plates").iter().map(|p| number(&p["mass"])))),
        ("Scharnieren", sum(&mut array(json, "hinges").iter().map(|h| number(&h["mass"])))),
    ];
    let mut table = String::from("| Omschrijving | Gewicht [kg] |\n|---|---:|\n");
    for (name, mass) in &rows {
        writeln!(table, "| {name} | {} |", nl(*mass, 1)).unwrap();
    }
    let total: f64 = rows.iter().map(|(_, mass)| mass).sum();
    writeln!(table, "| **Totaal** | **{}** |", nl(total, 1)).unwrap();
    writeln!(table, "\nIn de simulatie weegt de constructie {} kg.", nl(number(&json["total_mass"]), 1)).unwrap();
    table
}

fn strut_table(struts: &[Value]) -> String {
    let mut table = String::from(
        "| Staaf | Rustlengte [m] | Lengte staand [m] | Druk [kN] | Pennen | Afwijking [mm] |\n|---|---:|---:|---:|---|---:|\n",
    );
    for strut in struts {
        let set = &strut["tube_set"];
        let pins: Vec<&str> = array(set, "pins").iter().filter_map(Value::as_str).collect();
        writeln!(
            table,
            "| {} | {} | {} | {} | {} | {} |",
            strut["name"].as_str().unwrap_or_default(),
            nl(number(&strut["rest_length"]), 3),
            nl(number(&strut["length"]), 3),
            kn(-number(&strut["force"])),
            pins.join(" / "),
            nl(number(&set["length_error"]) * 1000.0, 0),
        )
        .unwrap();
    }
    table
}

fn soort(kind: &str) -> &'static str {
    match kind {
        "cable" => "Kabel 6 mm",
        "hanger" => "Gaffelstang, ophanging knoopplaat",
        "fork rod" => "Gaffelstang",
        other => panic!("unknown tension kind {other}"),
    }
}

fn tension_table(tension: &[Value]) -> String {
    let mut kinds: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for member in tension {
        kinds.entry(member["kind"].as_str().unwrap_or_default()).or_default().push(member);
    }
    let mut table = String::from(
        "| Trekelement | Aantal | Rustlengte [m] | Grootste trek [kN] | Slap staand |\n|---|---:|---|---:|---:|\n",
    );
    for (kind, members) in kinds {
        let forces: Vec<Option<f64>> = members.iter().map(|m| m["force"].as_f64()).collect();
        writeln!(
            table,
            "| {} | {} | {} | {} | {} |",
            soort(kind),
            members.len(),
            span(members.iter().map(|m| number(&m["rest_pin_to_pin"]))),
            kn(forces.iter().flatten().fold(0.0, |most: f64, &f| most.max(f))),
            forces.iter().filter(|f| f.is_none()).count(),
        )
        .unwrap();
    }
    table
}

fn feet_table(json: &Value) -> String {
    let mut table = String::from("| Voet | Staaf | x [m] | y [m] | Onderdeel |\n|---|---|---:|---:|---|\n");
    for foot in array(json, "feet") {
        writeln!(
            table,
            "| {} | {} | {} | {} | Stangkop INA GIKR12-PW |",
            foot["joint"].as_str().unwrap_or_default(),
            foot["strut"].as_str().unwrap_or_default(),
            nl(number(&foot["position"][0]), 3),
            nl(number(&foot["position"][1]), 3),
        )
        .unwrap();
    }
    table
}

fn guy_table(guy_lines: &[Value]) -> String {
    let mut table = String::from(
        "| Tuidraad | Vanaf | Anker x [m] | Anker y [m] | Rustlengte [m] | Trek staand [kN] |\n|---|---|---:|---:|---:|---:|\n",
    );
    for guy in guy_lines {
        writeln!(
            table,
            "| {} | {} | {} | {} | {} | {} |",
            guy["name"].as_str().unwrap_or_default(),
            guy["joint"].as_str().unwrap_or_default(),
            nl(number(&guy["anchor_position"][0]), 3),
            nl(number(&guy["anchor_position"][1]), 3),
            nl(number(&guy["rest_length"]), 3),
            kn(number(&guy["force"])),
        )
        .unwrap();
    }
    table
}
