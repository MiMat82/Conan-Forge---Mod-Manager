#[cfg(not(feature = "nexus-testing"))]
#[test]
fn public_build_rejects_nexus_api_actions_before_network_and_keeps_browser_import() {
    use conan_mod_manager_lib::{
        Action, Engine,
        integrations::{OnlineAction, nexus, site_url},
    };
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::open(root.path()).unwrap();
    let status = engine
        .dispatch(Action::Online {
            action: OnlineAction::Status,
        })
        .unwrap();
    assert_eq!(status["nexusApiEnabled"], false);
    assert_eq!(status["connected"], false);
    for action in [
        OnlineAction::Connect {
            key: "not-a-real-key-fixture".into(),
        },
        OnlineAction::Lookup {
            provider: "nexus".into(),
            input: "155".into(),
        },
        OnlineAction::Latest,
        OnlineAction::DownloadNexus {
            mod_id: "155".into(),
            file_id: "1".into(),
            nxm: String::new(),
        },
    ] {
        assert_eq!(
            engine.dispatch(Action::Online { action }).unwrap_err(),
            nexus::PUBLIC_NOTICE
        );
    }
    assert!(
        site_url("nexus", None)
            .unwrap()
            .starts_with("https://www.nexusmods.com/")
    );
    assert!(engine.store.acquisitions().unwrap().is_empty());
}
