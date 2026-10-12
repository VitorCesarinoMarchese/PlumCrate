//! Read-only metadata probe. Supply a local key through CURSEFORGE_API_KEY.
use ts4_mod_manager_core::{
    browser_import::load_browser_project,
    curseforge_client::{CurseForgeClient, UreqCurseForgeTransport},
    curseforge_details::{load_description, load_project_reference},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("Provide a CurseForge project ID")?
        .parse::<u64>()?;
    let key = std::env::var("CURSEFORGE_API_KEY")
        .map_err(|_| "Set a local test key in the environment")?;
    let client = CurseForgeClient::new(Some(&key), UreqCurseForgeTransport::default())
        .map_err(|_| "Missing test key")?;
    let project = load_browser_project(&client, id)
        .map_err(|_| "Could not read verified project releases")?;
    let description = load_description(&client, id, &project.source_url)
        .map_err(|_| "Could not read the author description")?;
    let dependencies = project
        .releases
        .first()
        .map(|f| f.dependencies.relations.as_slice())
        .unwrap_or_default();
    for dependency in dependencies {
        load_project_reference(&client, dependency.project_id)
            .map_err(|_| "Could not resolve a declared dependency")?;
    }
    println!(
        "{}",
        serde_json::json!({"projectId":id,"descriptionBlocks":description.blocks.len(),"authorLinkedProjects":description.author_projects().len(),"verifiedReleases":project.releases.len(),"resolvedDeclaredDependencies":dependencies.len()})
    );
    Ok(())
}
