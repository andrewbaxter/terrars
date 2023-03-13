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
};
use terrars_andrewbaxter_dinker::{
    BuildProviderDinker,
    BuildImage,
    BuildImageFilesEl,
    ImagePortsEl,
    BuildImagePortsEl,
};
use terrars_andrewbaxter_localrun::{
    BuildProviderLocalrun,
    BuildDataRun,
};
use terrars_fly_apps_fly::{
    BuildProviderFly,
    BuildApp,
    BuildIp,
    BuildCert,
    BuildMachine,
    BuildMachineServicesEl,
    BuildMachineServicesElPortsEl,
};
use terrars_hashicorp_aws::{
    BuildProviderAws,
    BuildRoute53Record,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let deploy_root = root.join("deploy");
    let tf_root = deploy_root.join("tf");
    fs::create_dir_all(&tf_root).unwrap();
    let stack = &mut BuildStack {}.build();

    // Input vars
    let aws_region = &BuildVariable { tf_id: "aws_region".into() }.build(stack);
    let fly_region = &BuildVariable { tf_id: "fly_region".into() }.build(stack);
    let domain = &BuildVariable { tf_id: "domain".into() }.build(stack);
    let domain_zone_id = &BuildVariable { tf_id: "domain_zone_id".into() }.build(stack);
    let aws_access_key = BuildVariable { tf_id: "aws_public_key".into() }.build(stack);
    let aws_secret_key = BuildVariable { tf_id: "aws_secret_key".into() }.build(stack).set_sensitive(true);
    let fly_token = BuildVariable { tf_id: "fly_token".into() }.build(stack).set_sensitive(true);

    // Auth
    BuildProviderLocalrun {}.build(stack);
    BuildProviderDinker {}
        .build(stack)
        .set_cache_dir(deploy_root.join("dinker_cache").to_string_lossy().to_string());
    BuildProviderAws {}
        .build(stack)
        .set_region(aws_region)
        .set_access_key(&aws_access_key)
        .set_secret_key(&aws_secret_key);
    BuildProviderFly {}
        .build(stack)
        .set_fly_api_token(&fly_token)
        .set_useinternaltunnel(true)
        .set_internaltunnelorg("personal")
        .set_internaltunnelregion(fly_region);

    // Fly setup
    let fly_token_str = fly_token.to_string();
    let fly_docker_creds = Some(("x", fly_token_str.as_str()));
    let fly_app = BuildApp {
        tf_id: "z2IL6YNID".into(),
        name: "terrars-helloworld".into(),
    }.build(stack);
    let app_ip = BuildIp {
        tf_id: "z57885UHY".into(),
        app: fly_app.id().into(),
        type_: "v4".into(),
    }.build(stack);
    let app_cert = BuildCert {
        tf_id: "zKUQBRBGM".into(),
        app: fly_app.id().into(),
        hostname: domain.into(),
    }.build(stack);
    BuildRoute53Record {
        tf_id: "z0CKHY0PF".into(),
        name: domain.into(),
        type_: "A".into(),
        zone_id: domain_zone_id.into(),
    }.build(stack).set_ttl(180f64).set_records(primvec![app_ip.address().to_string()]);
    BuildRoute53Record {
        tf_id: "zW5VYS124".into(),
        name: app_cert.dnsvalidationhostname().into(),
        type_: "CNAME".into(),
        zone_id: domain_zone_id.into(),
    }.build(stack).set_ttl(180f64).set_records(primvec![app_cert.dnsvalidationtarget().to_string()]);

    // Docker image
    let rust =
        BuildDataRun {
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
        let mut image =
            BuildImage {
                tf_id: "zN7CYROBV".into(),
                dest: format!(
                    "docker://registry.fly.io/{}:terrars-helloworld-{}-{}",
                    fly_app.name(),
                    tf_substr(bin_server_hash, 0, 8),
                    "{short_hash}"
                ).into(),
                files: vec![BuildImageFilesEl { source: bin_server.into() }.build().set_mode("0755")],
            }
                .build(stack)
                .set_arch("amd64")
                .set_os("linux")
                .set_cmd(primvec!["/helloworld"])
                .set_ports(
                    [80]
                        .into_iter()
                        .map(|p| BuildImagePortsEl { port: (p as f64).into() }.build())
                        .collect::<Vec<ImagePortsEl>>(),
                );
        if let Some((user, password)) = &fly_docker_creds {
            image = image.set_dest_user(*user).set_dest_password(*password);
        }
        image
    };

    // Fly machine
    BuildMachine {
        tf_id: "zIISRBA5Z".into(),
        image: stack.func("trimprefix").e(&image_app.rendered_dest()).l("docker://").into(),
        region: fly_region.into(),
    }
        .build(stack)
        .depends_on(&image_app)
        .set_name("main")
        .set_cputype("shared")
        .set_cpus(1f64)
        .set_memorymb(256f64)
        .set_app(fly_app.id())
        .set_services(vec![BuildMachineServicesEl {
            internal_port: 80f64.into(),
            ports: vec![BuildMachineServicesElPortsEl { port: 443f64.into() }.build().set_handlers(primvec!["tls"])],
            protocol: "tcp".into(),
        }.build()]);

    // Save the stack file
    fs::write(tf_root.join("stack.tf.json"), stack.serialize(&tf_root.join("state.json")).unwrap()).unwrap();
}
