use std::io::Read;

use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    noargs::HELP_FLAG.take_help(&mut args);
    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let count: usize = noargs::opt("count")
        .short('c')
        .ty("INTEGER")
        .default("1")
        .doc("Number of JSON values to generate")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let seed: Option<u64> = noargs::opt("seed")
        .short('s')
        .ty("INTEGER")
        .doc("Seed for the random number generator")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let mut rng = ChaChaRng::seed_from_u64(seed.unwrap_or_else(|| {
        std::time::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_millis() as u64
    }));

    let mut template_json_text = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut template_json_text)?;
    let (template_json, _) = nojson::RawJson::parse_jsonc(&template_json_text)
        .map_err(|e| rjg::json::format_parse_error(&template_json_text, e))?;
    let template_value = rjg::template::ValueTemplate::try_from(template_json.value())
        .map_err(|e| rjg::json::format_parse_error(&template_json_text, e))?;

    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    for _ in 0..count {
        template_value.generate(&mut writer, &mut rng)?;
    }

    Ok(())
}
