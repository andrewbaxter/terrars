TERRARUST is a set of ergonomic tools for building Terraform configs from Rust. This is an alternative to the CDK.

Benefits

- One language: If you're already using Rust, why deal with Node.js and other languages for deploys?
- Type safety, including resource property references
- _You_ control TERRARUST

# What it is

TERRARUST is a library with some data structures for describing and serializing Terraform stacks. `Stack` is the root data type.

TERRARUST also provides a command, `terrarust-generate`, which generates Rust code for provider-specific types. In the current directory, it creates a module for each provider you specify.

# Usage

1. Generate schemas for the providers you want. As an example, `andrewbaxter/stripe`

   You need to have `terraform` and `rustfmt` installed and on your path.

   Run `cargo install terrarust`, then `terrarust-generate andrewbaxter/stripe:0.0.14` (or whatever the latest version is).

   Copy `stripe/` into your project source tree somewhere and add `pub mod stripe` to the parent module.

2. Develop your code

   Create a `Stack` and set up the provider type and provider:

   ```
   let stack = BuildStack {
     state_path: PathBuf::from_str("mystack.tfstate").unwrap(),
   }::build();
   let pt = provider_type_stripe(&stack);
   BuildProviderStripe {
     provider_type: pt,
     token: STRIPE_TOKEN,
   }.build(&stack);
   ```

   Then create resources:

   ```
   let my_product = BuildProduct {
     name: "My Product".into(),
   }.build(&stack);
   ...
   ```

   Finally, write the stack out:

   ```
   fs::write("mystack.tf.json", stack.serialize())?;
   ```

3. Call `terraform` on your stack as usual
