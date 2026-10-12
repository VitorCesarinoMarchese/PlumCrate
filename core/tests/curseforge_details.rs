use serde_json::json;
use ts4_mod_manager_core::{
    curseforge_client::{CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError},
    curseforge_details::{self, BlockKind, SafeUrl},
};

#[test]
fn full_html_preserves_structure_entities_author_links_images_and_tables() {
    let html = r#"<h2>Requirements &amp; support</h2><p>Install the <strong>Core Library</strong> first. <a href='https://www.curseforge.com/sims4/mods/core-library'>Get the library</a></p><ol><li>Close the game</li><li>Enable script mods</li></ol><table><tr><th>Version</th><th>Game</th></tr><tr><td>1.0</td><td>1.128</td></tr></table><img src='https://example.org/instructions.png' alt='Installation instructions'><p><a href='https://example.org/support'>Support the author</a></p>"#;
    let doc = curseforge_details::parse_description_response(&json!({"data":html}).to_string(), "https://www.curseforge.com/sims4/mods/main").unwrap();
    let text = doc.blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("Requirements & support"));
    assert!(text.contains("Core Library"));
    assert!(text.contains("Enable script mods"));
    assert!(text.contains("Version | Game"));
    assert!(text.contains("1.0 | 1.128"));
    assert!(doc.blocks.iter().any(|b| b.kind == BlockKind::Heading(2)));
    assert!(doc.blocks.iter().any(|b| b.image.as_ref().is_some_and(|u|u.as_str().ends_with("instructions.png"))));
    assert_eq!(doc.author_projects().len(), 1);
    assert!(doc.blocks.iter().flat_map(|b|&b.links).any(|l|l.url.as_str()=="https://example.org/support"));
}

#[test]
fn executable_content_credentials_and_unsafe_schemes_never_reach_the_document() {
    let html = r#"<script>secret-script</script><style>secret-style</style><iframe>hidden-frame</iframe><p>Safe <a href='javascript:alert(1)'>text</a><a href='file:///etc/passwd'>local</a><a href='https://user:password@example.org/'>credentials</a><a href='/sims4/mods/real'>Real</a></p>"#;
    let doc=curseforge_details::parse_description_response(&json!({"data":html}).to_string(), "https://www.curseforge.com/sims4/mods/main").unwrap();
    let text=doc.blocks.iter().map(|b|b.text.as_str()).collect::<Vec<_>>().join(" ");
    assert!(!text.contains("secret") && !text.contains("hidden-frame"));
    assert!(text.contains("Safe text"));
    let links=doc.blocks.iter().flat_map(|b|&b.links).collect::<Vec<_>>();
    assert_eq!(links.len(),1);
    assert_eq!(links[0].url.as_str(),"https://www.curseforge.com/sims4/mods/real");
    assert!(SafeUrl::parse("https://www.curseforge.com.attacker/sims4/mods/real").unwrap().project_slug().is_none());
}

#[test]
fn invalid_or_oversized_descriptions_fail_instead_of_silently_truncating() {
    for data in [json!({"data":42}),json!({"data":"x".repeat(2_100_000)})] {
        assert_eq!(curseforge_details::parse_description_response(&data.to_string(),"https://www.curseforge.com/sims4/mods/main"),Err(SourceLookupError::InvalidResponse));
    }
    let doc=curseforge_details::parse_description_response(&json!({"data":"<p></p>"}).to_string(),"https://www.curseforge.com/sims4/mods/main").unwrap();
    assert!(doc.blocks.is_empty());
}

struct Provider { mismatched:bool }
impl CurseForgeTransport for Provider {
    fn get(&self, r:&CurseForgeRequest)->Result<(u16,String),SourceLookupError> {
        if r.url.contains("/search?") {
            assert!(r.url.contains("slug=core-library"));
            return Ok((200,json!({"data":[{"id":9,"gameId":78062,"name":"Library","slug":"core-library","links":{"websiteUrl":if self.mismatched {"https://www.curseforge.com/sims4/mods/other"} else {"https://www.curseforge.com/sims4/mods/core-library"}},"authors":[]}]}).to_string()));
        }
        Ok((200,json!({"data":{"id":9,"gameId":78062,"name":"Library","links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/core-library"}}}).to_string()))
    }
}
#[test]
fn author_link_resolution_requires_exact_provider_url_and_real_project_identity() {
    let link=SafeUrl::parse("https://www.curseforge.com/sims4/mods/core-library/").unwrap();
    let client=CurseForgeClient::new(Some("synthetic"),Provider{mismatched:false}).unwrap();
    assert_eq!(curseforge_details::resolve_author_project(&client,&link).unwrap().project_id,9);
    let client=CurseForgeClient::new(Some("synthetic"),Provider{mismatched:true}).unwrap();
    assert!(curseforge_details::resolve_author_project(&client,&link).is_err());
    assert!(curseforge_details::load_project_reference(&client,10).is_err());
}
