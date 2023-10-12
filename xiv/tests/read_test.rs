const SQPACK_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/../sqpack");

fn open() -> std::sync::Arc<xiv::sqpack::SqPack> {
    std::fs::read_dir(SQPACK_PATH).expect(&format!("Tests expect SqPack repo at {SQPACK_PATH}"));

    xiv::sqpack::SqPack::open(SQPACK_PATH).unwrap()
}

#[test]
fn read_text() {
    let repo = open();

    let file = repo
        .find("exd/root.exl")
        .unwrap()
        .expect("Failed to find root.exl");

    let raw = file.read_plain().unwrap();
    let utf8 = std::str::from_utf8(&raw).expect("Failed to recode root.exl");

    assert!(utf8.contains("Race"));
    assert!(utf8.contains("ModelChara"));
    assert!(utf8.contains("Items"));
    assert!(utf8.contains("Action"));
}

#[test]
fn read_exd_using_row() {
    let repo = open();

    let races: Vec<xiv::ex::Row> = xiv::ex::read_exd(repo.clone(), "Race", xiv::ex::Locale::English)
        .expect("Failed to find Race.exd")
        .map(Result::unwrap)
        .collect();
    races
        .iter()
        .find(|r| r.get(1) == Some(&xiv::ex::Value::String("Hyur".into())))
        .expect("Race.exd should contain Hyur");
    races
        .iter()
        .find(|r| r.get(2) == Some(&xiv::ex::Value::String("Miqo'te".into())))
        .expect("Race.exd should contain Miqo'te");
}

#[test]
fn read_exd_using_struct() {
    let repo = open();

    let races: Vec<xiv::structs::Race> = xiv::ex::read_exd(repo.clone(), "Race", xiv::ex::Locale::English)
        .expect("Failed to find Race.exd")
        .map(Result::unwrap)
        .collect();
    races
        .iter()
        .find(|r| r.masculine == "Hyur")
        .expect("Race.exd should contain Hyur");
    races
        .iter()
        .find(|r| r.feminine == "Miqo'te")
        .expect("Race.exd should contain Miqo'te");
}

#[test]
fn export_image() {
    use image::GenericImageView;

    let repo = open();
    let white = "chara/common/texture/white.tex";
    let black = "chara/common/texture/black.tex";

    let file = repo
        .find(white)
        .unwrap()
        .expect(&format!("Failed to find {white}"));
    let src_image = file.read_image().unwrap();
    let exp_image = src_image.export().unwrap();
    for (_x, _y, image::Rgba(color)) in exp_image.pixels() {
        assert_eq!(color, [255, 255, 255, 255], "{white} is not white");
    }

    let file = repo
        .find(black)
        .unwrap()
        .expect(&format!("Failed to find {black}"));
    let src_image = file.read_image().unwrap();
    let exp_image = src_image.export().unwrap();
    for (_x, _y, image::Rgba(color)) in exp_image.pixels() {
        assert_eq!(color, [0, 0, 0, 255], "{black} is not black");
    }
}
