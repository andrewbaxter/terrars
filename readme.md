Terrars is a set of ergonomic tools for building Terraform configs from Rust. This is an alternative to the CDK.

Benefits

- Share definitions with your code (like endpoints, names, ids)
- Type safety - including resource property references
- Fewer layers - just your code and Terraform

Current status: Usable, but may have some rough edges and missing features. Let me know if you encounter issues.

# What it is

Terrars is a library with some data structures for describing and serializing Terraform stacks. `Stack` is the root data type.

Terrars also provides a command, `terrars-generate`, which generates Rust code for provider-specific types. In the current directory, it creates a module for each provider you specify.

# Usage

1. Generate schemas for the providers you want. As an example, `andrewbaxter/stripe`

   You need to have `terraform` and `rustfmt` installed and on your path.

   Run `cargo install terrars`, then `terrars-generate andrewbaxter/stripe 0.0.14` (or whatever the latest version is).

   Copy `stripe/` into your project source tree somewhere and add `pub mod stripe` to the parent module.

2. Develop your code

   Create a `Stack` and set up provider types and provider:

   ```rust
   let mut stack = BuildStack {
       state_path: PathBuf::from_str("mystack.tfstate").unwrap(),
   }.build();
   let pt = provider_type_stripe(&mut stack);
   BuildProviderStripe {
       provider_type: pt,
       token: STRIPE_TOKEN,
   }.build(&mut stack);
   ```

   Then create resources:

   ```rust
   let my_product = BuildProduct {
       name: "My Product".into(),
   }.build(&mut stack);
   let my_price = BuildPrice {
       ...
   }.build(&mut stack);
   my_price.set_product(my_product.id());
   ...
   ```

   Finally, write the stack out:

   ```rust
   fs::write("mystack.tf.json", stack.serialize()?)?;
   ```

3. Call `terraform` on your stack as usual

   `Stack` also has methods `run()` and `get_output()` to call `terraform` for you. You must have `terraform` in your path.

# Data model

The base library has `BuildStack`, `BuildVariable` and `BuildOutput` structs for creating those three items.

`terrars-generate` creates `provider_type_*()`, `BuildProvider*`, and `BuildResource*`/`BuildData*` for you for all resources/datasources in the provider.

In general `Build*` structs have required fields and a `build()` method that makes the object usable and registers it with the `Stack`.

# Limitations

- Limited name sanitization

  If a provider field or resource name clashes with a rust keyword or metavalue name things may fail to compile.

- Not all Terraform features have been implemented

  The only ones I'm aware of missing at the moment are resource "provisioning" and for-each/count.

- References for collection types are not implemented

  I don't know if those are generally valid in the first place, outside of for-each/count.

- `ignore_changes` takes strings rather than an enum

- No variable or output static type checking

  I'd like to add a derive macro for generating variables/outputs automatically from a structure at some point.
