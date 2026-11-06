use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use akibiyori::{BuildOptions, build, generate};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn route_source(source_root: &Path, manifest: &Value, route: &str) -> String {
    let path = manifest["content_source_files"][route]
        .as_str()
        .unwrap_or_else(|| panic!("generated source route {route:?} exists"));
    fs::read_to_string(source_root.join(path)).expect("read generated source page")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn native_program(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name).map_or_else(|| PathBuf::from(fallback), PathBuf::from)
}

#[test]
fn pinned_petstore_generates_source_facts_and_real_native_routes() {
    let root = repository_root();
    let fixture = root.join("examples/petstore/contracts/openapi.yaml");
    let license = root.join("examples/petstore/LICENSE");
    assert_eq!(
        sha256(&fs::read(&fixture).expect("read pinned Petstore YAML")),
        "0d810997f6409d5cff6f0cf2c1466814ba52250a784cd841cacb93514c7a8502"
    );
    assert_eq!(
        sha256(&fs::read(&license).expect("read Petstore license")),
        "b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1"
    );

    let temp = TempDir::new().expect("Petstore output root");
    let config = root.join("examples/petstore/config.json");
    let source_root = temp.path().join("source");
    let generation = generate(&config, &source_root, "").expect("generate Petstore source");
    assert_eq!(generation.authored_pages, 3);
    assert_eq!(generation.operation_pages, 19);
    assert_eq!(generation.model_pages, 6);

    let manifest: Value = serde_json::from_slice(
        &fs::read(source_root.join("_data/akibiyori-manifest.json"))
            .expect("generated source manifest"),
    )
    .expect("manifest is JSON");
    let operation_ids: BTreeSet<_> = manifest["operation_routes"]
        .as_object()
        .expect("operation route map")
        .keys()
        .map(String::as_str)
        .collect();
    let expected_operations = [
        "updatePet",
        "addPet",
        "findPetsByStatus",
        "findPetsByTags",
        "getPetById",
        "updatePetWithForm",
        "deletePet",
        "uploadFile",
        "getInventory",
        "placeOrder",
        "getOrderById",
        "deleteOrder",
        "createUser",
        "createUsersWithListInput",
        "loginUser",
        "logoutUser",
        "getUserByName",
        "updateUser",
        "deleteUser",
    ]
    .into_iter()
    .collect();
    assert_eq!(operation_ids, expected_operations);
    assert_eq!(
        manifest["operation_routes"]["getPetById"],
        "/reference/getPetById/"
    );

    let model_ids: BTreeSet<_> = manifest["model_routes"]
        .as_object()
        .expect("model route map")
        .keys()
        .map(String::as_str)
        .collect();
    let expected_models = [
        "petstore:Order",
        "petstore:Category",
        "petstore:User",
        "petstore:Tag",
        "petstore:Pet",
        "petstore:ApiResponse",
    ]
    .into_iter()
    .collect();
    assert_eq!(model_ids, expected_models);
    assert_eq!(
        manifest["model_routes"]["petstore:Pet"],
        "/reference/models/petstore/Pet/"
    );

    let generated_operation = route_source(&source_root, &manifest, "/reference/getPetById/");
    for fact in [
        "GET</span><code>/pet/{petId}</code>",
        "<code>petId</code> <span>path</span>",
        "<p>Required</p>",
        "int64",
        "data-choice-label=\"api_key\"",
        "data-choice-label=\"petstore_auth\"",
        "write:pets",
        "read:pets",
        "data-choice-label=\"200\"",
        "data-choice-label=\"400\"",
        "data-choice-label=\"404\"",
        "data-choice-label=\"default\"",
        "application/json",
        "application/xml",
    ] {
        assert!(
            generated_operation.contains(fact),
            "missing operation fact {fact:?}"
        );
    }

    let pet_page = route_source(&source_root, &manifest, "/reference/models/petstore/Pet/");
    for fact in [
        "data-property-name=\"name\"",
        "data-property-name=\"photoUrls\"",
        "photoUrls",
        "data-property-name=\"status\"",
        "available",
        "pending",
        "sold",
        "/reference/models/petstore/Category/",
        "/reference/models/petstore/Tag/",
    ] {
        assert!(pet_page.contains(fact), "missing Pet schema fact {fact:?}");
    }
    let name_start = pet_page
        .find("data-property-name=\"name\"")
        .expect("name field exists");
    let name_tail = &pet_page[name_start..];
    let name_end = name_tail.find("</section>").expect("name field closes");
    assert!(name_tail[..name_end].contains("<span class=\"required\">required</span>"));
    let photo_start = pet_page
        .find("data-property-name=\"photoUrls\"")
        .expect("photoUrls field exists");
    let photo_tail = &pet_page[photo_start..];
    let photo_end = photo_tail
        .find("</section></section>")
        .map(|offset| offset + "</section></section>".len())
        .expect("photoUrls field and its array items close");
    let photo_field = &photo_tail[..photo_end];
    assert!(photo_field.contains("photoUrls <span class=\"required\">required</span>"));
    assert!(photo_field.contains("<strong>type</strong>: array"));
    assert!(photo_field.contains("<section><h2>Array items</h2><p><strong>type</strong>: string"));
    let category_start = pet_page
        .find("data-property-name=\"category\"")
        .expect("category field exists");
    let category_tail = &pet_page[category_start..];
    let category_end = category_tail
        .find("</section></section>")
        .map(|offset| offset + "</section></section>".len())
        .expect("category field and nested properties close");
    assert!(category_tail[..category_end].contains("/reference/models/petstore/Category/"));
    let tags_start = pet_page
        .find("data-property-name=\"tags\"")
        .expect("tags field exists");
    let tags_tail = &pet_page[tags_start..];
    let tags_end = tags_tail
        .find("</section></section>")
        .map(|offset| offset + "</section></section>".len())
        .expect("tags field and array items close");
    assert!(tags_tail[..tags_end].contains("/reference/models/petstore/Tag/"));
    let status_start = pet_page
        .find("data-property-name=\"status\"")
        .expect("status field exists");
    let status_tail = &pet_page[status_start..];
    let status_end = status_tail.find("</section>").expect("status field closes");
    for value in ["available", "pending", "sold"] {
        assert!(
            status_tail[..status_end].contains(value),
            "missing Pet enum {value}"
        );
    }
    let order_page = route_source(&source_root, &manifest, "/reference/models/petstore/Order/");
    assert!(order_page.contains("data-property-name=\"shipDate\""));
    let ship_start = order_page
        .find("data-property-name=\"shipDate\"")
        .expect("shipDate property exists");
    let ship_tail = &order_page[ship_start..];
    let ship_end = ship_tail.find("</section>").expect("shipDate field closes");
    let ship_field = &ship_tail[..ship_end];
    assert!(ship_field.contains("<strong>type</strong>: string"));
    assert!(ship_field.contains("date-time"));
    for status in ["placed", "approved", "delivered"] {
        assert!(order_page.contains(status), "missing Order status {status}");
    }
    let inventory = route_source(&source_root, &manifest, "/reference/getInventory/");
    assert!(inventory.contains("Additional property values"));
    assert!(inventory.contains("integer"));
    let upload = route_source(&source_root, &manifest, "/reference/uploadFile/");
    assert!(upload.contains("application/octet-stream"));
    assert!(upload.contains("data-example-unavailable=\"true\""));

    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real Cobalt and Pagefind Petstore build");
    assert_eq!(report.generation.authored_pages, 3);
    assert_eq!(report.generation.operation_pages, 19);
    assert_eq!(report.generation.model_pages, 6);
    let operation = fs::read_to_string(report.served_dir.join("reference/getPetById/index.html"))
        .expect("native operation page");
    assert!(operation.contains("<h3><code>petId</code> <span>path</span></h3>"));
    assert!(operation.contains("data-choice-label=\"api_key\""));
    assert!(operation.contains("data-choice-label=\"petstore_auth\""));
    assert!(operation.contains("data-media-label=\"application/json\""));
    assert!(operation.contains("data-media-label=\"application/xml\""));
    let pet = fs::read_to_string(
        report
            .served_dir
            .join("reference/models/petstore/Pet/index.html"),
    )
    .expect("native Pet model page");
    let photo_id = "field-232f636f6d706f6e656e74732f736368656d61732f5065742f70726f706572746965732f70686f746f55726c73";
    assert!(pet.contains(&format!("id=\"{photo_id}\"")));
    let photo_start = pet
        .find("data-property-name=\"photoUrls\"")
        .expect("native photoUrls field");
    let photo_tail = &pet[photo_start..];
    let photo_end = photo_tail
        .find("</section></section>")
        .map(|offset| offset + "</section></section>".len())
        .expect("native photoUrls field closes");
    let photo_field = &photo_tail[..photo_end];
    assert!(photo_field.contains("<span class=\"required\">required</span>"));
    assert!(photo_field.contains("<p><strong>type</strong>: array</p>"));
    assert!(
        photo_field.contains(
            "<section><h2>Array items</h2><p><strong>type</strong>: string</p></section>"
        )
    );
    let search_entry: Value = serde_json::from_slice(
        &fs::read(report.served_dir.join("pagefind/pagefind-entry.json"))
            .expect("native Pagefind entry"),
    )
    .expect("Pagefind entry JSON");
    assert_eq!(search_entry["version"], "1.5.2");
    assert_eq!(search_entry["languages"]["en"]["page_count"], 30);
}
