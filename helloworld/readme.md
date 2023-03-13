This example shows how to deploy a simple Rust webserver on Fly and AWS.

**Warning** the Fly Terraform provider is largely unmaintained and has numerous bugs at the moment. The initial deploy should hopefully work, but if you run into issues you may need to look for patches in the pull requests there.

The Terraform stack generation is done in `build.rs`.

This requires the `musl` target to create binary to add to the docker image. You can set up that target by running `rustup target add x86_64-unknown-linux-musl`.

To deploy the example:

1. Do `cargo build` if the stack file isn't already generated
2. Move to `deploy/tf`
3. Create `input.json` with the following contents:

   ```json
   {
     "aws_region": "us-east-1",
     "fly_region": "ord",
     "domain": "helloworld.example.com",
     "domain_zone_id": "Z...",
     "aws_public_key": "AKIA...",
     "aws_secret_key": "...",
     "fly_token": "..."
   }
   ```

   (replacing the example values above with your own)

4. Run `terraform init` then `terraform apply --var-file input.json`

To update the deployment, just run `terraform apply ...` again. To destroy the website do `terraform destroy ...`.
