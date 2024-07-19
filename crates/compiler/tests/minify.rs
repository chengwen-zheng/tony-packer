use std::collections::HashMap;

use toy_farm_testing_helpers::fixture;
mod common;
use common::create_compiler;

#[tokio::test]
async fn minify_script_test() {
    fixture!(
        "tests/fixtures/minify/script/**/index.ts",
        |file, crate_path| async move {
            let cwd = file.parent().unwrap();
            println!("testing minify: {:?}", cwd);

            let entry_name = "index".to_string();
            let _compiler = create_compiler(
                HashMap::from([(entry_name.clone(), "./index.ts".to_string())]),
                cwd.to_path_buf(),
                crate_path,
                true,
            );

            // compiler.compile().await

            //   assert_compiler_result(&compiler, Some(&entry_name));
        }
    );
}
