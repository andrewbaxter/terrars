use std::{
    path::PathBuf,
    env,
    fs,
};
use terrars::{
    BuildStack,
    BuildVariable,
    primvec,
    tf_substr,
    tf_trim_prefix,
};
use terrars_andrewbaxter_dinker::{
    BuildProviderDinker,
    BuildImage,
    BuildImageFilesEl,
};
use terrars_andrewbaxter_localrun::{
    BuildProviderLocalrun,
    BuildDataAlwaysRun,
};
use terrars_andrewbaxter_fly::{
    BuildProviderFly,
    BuildApp,
    BuildMachine,
    BuildMachineServicesEl,
    BuildMachineServicesElPortsEl,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let deploy_root = root.join("deploy");
    let tf_root = deploy_root.join("tf");
    fs::create_dir_all(&tf_root).unwrap();
    let stack = &mut BuildStack {}.build();

    // Input vars
    let fly_region = &BuildVariable { tf_id: "fly_region".into() }.build(stack);
    let fly_token = BuildVariable { tf_id: "fly_token".into() }.build(stack).set_sensitive(true);

    // Auth
    BuildProviderLocalrun {}.build(stack);
    BuildProviderDinker {}
        .build(stack)
        .set_cache_dir(deploy_root.join("dinker_cache").to_string_lossy().to_string());
    BuildProviderFly {}.build(stack).set_fly_api_token(&fly_token);

    // Fly setup
    let fly_token_str = fly_token.to_string();
    let fly_docker_creds = Some(("x", fly_token_str.as_str()));
    let fly_app = BuildApp {
        tf_id: "z2IL6YNID".into(),
        name: "terrars-helloworld".into(),
    }.build(stack);

    // Docker image
    let rust =
        BuildDataAlwaysRun {
            tf_id: "z22WPM6IT".into(),
            command: primvec![
                "cargo",
                "build",
                "--target=x86_64-unknown-linux-musl",
                "--bin=helloworld",
                "--release"
            ].into(),
        }
            .build(stack)
            .set_working_dir(root.to_str().unwrap())
            .set_outputs(
                primvec![root.join("../target/x86_64-unknown-linux-musl/release/helloworld").to_str().unwrap()],
            );
    let bin_server = rust.outputs().get(0);
    let bin_server_hash = rust.output_hashes().get(0);
    let image_app = {
        let mut image = BuildImage {
            tf_id: "zN7CYROBV".into(),
            dest: format!(
                "docker://registry.fly.io/{}:terrars-helloworld-{}-{}",
                fly_app.name(),
                tf_substr(stack, bin_server_hash, 0, 8),
                "{short_hash}"
            ).into(),
            files: vec![BuildImageFilesEl { source: bin_server.into() }.build().set_mode("0755")],
        }.build(stack).set_arch("amd64").set_os("linux").set_cmd(primvec!["/helloworld"]);
        if let Some((user, password)) = &fly_docker_creds {
            image = image.set_dest_user(*user).set_dest_password(*password);
        }
        image
    };

    // Fly machine
    BuildMachine {
        tf_id: "zIISRBA5Z".into(),
        app: fly_app.id().into(),
        image: tf_trim_prefix(stack, image_app.rendered_dest(), "docker://".to_string()).into(),
        region: fly_region.into(),
    }
        .build(stack)
        .depends_on(&image_app)
        .set_name("main")
        .set_cpu_type("shared")
        .set_cpus(1f64)
        .set_memory(256f64)
        .set_services(vec![BuildMachineServicesEl {
            internal_port: 53f64.into(),
            ports: vec![
                BuildMachineServicesElPortsEl { port: 10053f64.into() }.build(),
                BuildMachineServicesElPortsEl { port: 53f64.into() }.build()
            ],
            protocol: "udp".into(),
        }.build()]);

    // Save the stack file
    fs::write(tf_root.join("stack.tf.json"), stack.serialize(&tf_root.join("state.json")).unwrap()).unwrap();
}
