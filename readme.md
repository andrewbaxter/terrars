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

1. Generate schemas for the providers you want. As an example, `hashicorp/aws`

   You need to have `terraform` installed and on your path.

   First create a json file (ex: `terrars_aws.json`) containing the generation configuration:

   ```
   {
       "provider": "hashicorp/aws",
       "version": "4.48.0",
       "include": [
           "cognito_user_pool",
           "cognito_user_pool_client",
           "cognito_user_pool_domain",
           "cognito_user_pool_ui_customization",
           "route53_zone",
           "route53_record",
           "aws_acm_certificate",
           "aws_acm_certificate_validation"
       ],
       "dest": "src/bin/mydeploy/tfschema/aws"
   }
   ```

   `tfschema/aws` must be an otherwise unused directory - it will be wiped when you genenerate the code.

   Run `cargo install terrars`, then `terrars-generate terrars_aws.json`.

   The first time you do this, create a `src/bin/mydeploy/tfschema/mod.rs` file with this contents to root the generated provider:

   ```
   pub mod aws;
   ```

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

# How it works

Terraform provides a method to output provider schemas as json. This tool uses that schema to generate structures that would output matching json Terraform stack files.

## Expressions/template strings/interpolation/escaping

Take as an example:

```
format!("{}{}", my_expr, verbatim_string))
```

This code would somehow need to escape the pattern and `verbatim_string`, while leaving `my_expr` unescaped, and the result would need to be treated as an "expression" to prevent escaping if it's used again in another `format!` or something. This applies to not just `format!` but serde serialization (json), other methods.

For now Terrars uses a simple (somewhat dirty) hack to avoid this. All expressions are put into a replacement table, and a sentinel string (ex: `_TERRARS_SENTINEL_99_`) is used instead. During final stack json serialization, the values are escaped and then the sentinel values are substituted back out for the original expressions.

This way, all normal string formatting methods should retain the expected expressions.

# Current limitations and warnings

- Not all Terraform features have been implemented

  The only one I'm aware of missing at the moment is resource "provisioning".

- `ignore_changes` takes strings rather than an enum

- `for_each` doesn't have type mapping capabilities

- No variable or output static type checking

  I'd like to add a derive macro for generating variables/outputs automatically from a structure at some point.
