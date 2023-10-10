use anyhow::anyhow;
use clap::{Parser, Subcommand};
use fallible_iterator::{FallibleIterator, IteratorExt};
use std::{fs, path::Path, sync::Arc};
use xiv::{
    ex::{read_exd, Locale, Row},
    sqpack::SqPack,
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to "sqpack" directory
    #[arg(short, long, value_name = "SQPACK_DIR")]
    repo_dir: Box<Path>,

    /// Directory path to write exported files into
    #[arg(short, long)]
    out_dir: Option<Box<Path>>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List things within SqPack repository
    #[command(subcommand)]
    List(ListCommands),

    /// Export things from SqPack repository
    #[command(subcommand)]
    Export(ExportCommands),
}

#[derive(Subcommand)]
enum ListCommands {
    /// List all .exd files referenced by rool.exl
    Exd,
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export .exd → .csv
    Exd {
        /// Export only specific file by base name (e.g. "ModelChara")
        #[arg(short, long)]
        filter: Option<Box<str>>,
    },
    /// Export .tex -> .png/.jpg/.tga
    Tex {
        /// Target .tex file within SqPack repository
        path: Box<str>,
        /// Export file format
        #[arg(short, long, default_value = "png")]
        format: Box<str>,
    }
}

fn read_root_exl(repo: Arc<SqPack>) -> anyhow::Result<Vec<Box<str>>> {
    let root_path = "exd/root.exl";
    let root = repo
        .find(root_path)?
        .ok_or(anyhow!("{root_path} not found within SqPack repository"))?
        .read_plain()?;

    csv::Reader::from_reader(root.as_ref())
        .records()
        .transpose_into_fallible()
        .map_err(From::from)
        .map(|r| r.get(0).map(Box::from).ok_or(anyhow!("Malformed root.exl")))
        .collect()
}

fn list_exd(repo: Arc<SqPack>) -> anyhow::Result<()> {
    for sheet_name in read_root_exl(repo.clone())? {
        println!("{}", sheet_name);
    }
    Ok(())
}

fn export_one_exd(repo: Arc<SqPack>, out_dir: &Path, sheet_name: &str) -> anyhow::Result<()> {
    let rows: Vec<Row> = read_exd(repo.clone(), &sheet_name, Locale::English)?
        .transpose_into_fallible()
        .collect()?;

    let out_path = out_dir.join(sheet_name).with_extension("csv");

    fs::create_dir_all(out_path.parent().unwrap())?;
    let mut out_file = fs::File::create(&out_path)?;

    let mut w = csv::Writer::from_writer(&mut out_file);
    if let Some(first_row) = rows.first() {
        w.write_record(first_row.iter().map(|c| c.type_tag()))?;
    }
    for row in rows.iter() {
        w.serialize(&row)?;
    }
    w.flush()?;

    println!("{}", out_path.to_string_lossy());
    Ok(())
}

fn export_all_exd(repo: Arc<SqPack>, out_dir: &Path) -> anyhow::Result<()> {
    for sheet_name in read_root_exl(repo.clone())? {
        export_one_exd(repo.clone(), out_dir, &sheet_name)?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let repo = SqPack::open(cli.repo_dir)?;

    match cli.command {
        Commands::List(sub) => match sub {
            ListCommands::Exd => list_exd(repo.clone()),
        },
        Commands::Export(sub) => {
            let out_dir = cli
                .out_dir
                .ok_or(anyhow!("--out-dir is required for export commands"))?;

            match sub {
                ExportCommands::Exd { filter } => match filter {
                    Some(f) => export_one_exd(repo.clone(), &out_dir, &f),
                    None => export_all_exd(repo.clone(), &out_dir),
                },
                ExportCommands::Tex { path, format } => {
                    let path = path.to_lowercase();
                    let image = repo.find(&path)?.ok_or(anyhow!("{path} not found"))?.read_image()?;
                    let out_path = out_dir.join(&path).with_extension(format.as_ref());

                    fs::create_dir_all(out_path.parent().unwrap())?;
                    image.export()?.save(out_path)?;
                    Ok(())
                }
            }
        }
    }
}
