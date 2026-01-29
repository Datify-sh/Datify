use std::env;
use std::path::PathBuf;

use datify::generate_openapi_json;

fn main() {
    let args: Vec<String> = env::args().collect();

    let output_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("openapi.json")
    };

    match generate_openapi_json(&output_path) {
        Ok(_) => {
            println!("✓ OpenAPI specification generated successfully!");
            println!("  Location: {}", output_path.display());
            println!("\nYou can now:");
            println!("  • View it with a Swagger/OpenAPI viewer");
            println!("  • Import it into API clients (Postman, Insomnia, etc.)");
            println!("  • Generate client SDKs using openapi-generator");
        },
        Err(e) => {
            eprintln!("✗ Error generating OpenAPI specification: {}", e);
            std::process::exit(1);
        },
    }
}
