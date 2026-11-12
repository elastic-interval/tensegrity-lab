//! Headless Hug's handoff package: one folder with everything for
//! the engineer and the makers, from one build of the current plan.
//!
//! ```
//! cargo test --release --lib export_headless_hug_package -- --ignored
//! ```

use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::bouwboek;
use crate::connector::{dxf, welding};
use crate::fabric_json::fabric_json;
use crate::wgpu::figure;
use std::path::Path;
use std::fs;

/// The engineer's RFEM script, kept in the repository and shipped as it is.
const RFEM_SCRIPT: &str = "scripts/rfem/build_rfem.py";

const README: &str = "\
For the engineer
  {json}
      Headless Hug standing: every strut and its tubes, cable, fork rod, ring,
      plate, hinge, foot and guy line, each with its length, rest length, force
      and mass.
  build_rfem.py
      Reads the JSON and builds the model in RFEM 6:
          python build_rfem.py {json} --dry-run
      first. It starts with what it does and the settings to change.
  {bouwboek}
      The Bouwboek in draft, in Dutch: ENS's chapters, the numbers filled in
      from this build, the rest marked TE DOEN.

For the makers
  {parts}/
      Laser-cut parts, one DXF each, thickness and quantity in the file name.
  {welding}
      What is welded, with an estimate of the work and its cost.
  images/
      The figure from four sides, and each part on the parts list, as the
      app draws them, on white. The Bouwboek shows them.

Everything is from the same moment: the figure standing, as the simulation
settles it. Parts are made to rest lengths, which the JSON gives beside the
standing ones.
";

#[test]
#[ignore]
fn export_headless_hug_package() {
    let date = chrono::Local::now().format("%Y-%m-%d");
    let folder = format!("HeadlessHug-{date}");
    let json = format!("HeadlessHug-{date}.json");
    let parts = format!("HeadlessHug-parts-{date}");
    let welding = format!("HeadlessHug-welding-{date}.md");
    let book = format!("HeadlessHug-bouwboek-{date}.md");
    fs::remove_dir_all(&folder).ok();
    fs::create_dir_all(format!("{folder}/{parts}")).expect("create folder");

    let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    executor.fabric.update_all_attachment_connections();
    let value = serde_json::to_value(fabric_json(&executor.fabric)).expect("serialize");
    let text = serde_json::to_string_pretty(&value).expect("serialize");
    fs::write(format!("{folder}/{json}"), text).expect("write JSON");
    fs::copy(RFEM_SCRIPT, format!("{folder}/build_rfem.py")).expect("copy RFEM script");

    for (drawing, quantity) in dxf::drawings(&executor.fabric) {
        fs::write(format!("{folder}/{parts}/{}", drawing.file_name(quantity)), drawing.dxf()).expect("write DXF");
    }
    fs::write(format!("{folder}/{welding}"), welding::document(&executor.fabric)).expect("write welding");
    let figures = figure::write_figures(&executor.fabric, &Path::new(&folder).join("images"));
    let template = fs::read_to_string(bouwboek::TEMPLATE).expect("read Bouwboek template");
    fs::write(format!("{folder}/{book}"), bouwboek::document(&executor.fabric, &value, &figures, &template))
        .expect("write Bouwboek");

    let readme = README.replace("{json}", &json).replace("{parts}", &parts).replace("{welding}", &welding).replace("{bouwboek}", &book);
    fs::write(format!("{folder}/README.txt"), readme).expect("write README");

    println!("{folder}");
}
