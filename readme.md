Terrars is a tool for building Terraform stacks in Rust. This is an alternative to the [CDK](https://developer.hashicorp.com/terraform/cdktf).

**See a working example** in [helloworld](helloworld).

**Current status**: Usable, but may have some rough edges and missing features. I may continue to tweak things to improve ergonomics.

Why use this or the CDK instead of raw Terraform?

- All stacks eventually get complicated to the point you need a full programming language to generate them. CDK and this aren't particularly more verbose than raw Terraform, so I'd always use them from the start.
- Reuse constants and datastructures from your code (json structures, environment variables, endpoints and reverse routing functions) when defining your infrastructure
- Autocompletion, edit-time verification via types

Why use this instead of the CDK?

- It's Rust, which you're already using
- More static type safety - the CDK ignores types in a number of situations and munges required and optional parameters together
- Pre-generated bindings
- No complicated type hierarchy with scopes, inheritance, etc.
- Fewer layers - `cdk` requires `terraform`, a `cdk` CLI, Javascript tools, Javascript package directories, and depending on what language you use that language itself as well. CDK generation requires a `json spec -> typescript -> generated javascript -> final language` translation process. `terrars` only requires `terraform` both during generation and runtime and goes directly from the JSON spec to Rust.

Why _not_ use this instead of the CDK?

- You need to create your own workflow. You can create a simple build.rs file, but if you want a more complex wrapper you need to write it yourself.

# Pre-generated bindings

- [andrewbaxter/dinker](https://github.com/andrewbaxter/terrars-andrewbaxter-dinker) - Lightweight Docker image building
- [andrewbaxter/localrun](https://github.com/andrewbaxter/terrars-andrewbaxter-localrun) - External build scripts
- [andrewbaxter/stripe](https://github.com/andrewbaxter/terrars-andrewbaxter-stripe)
- [backblaze/b2](https://github.com/andrewbaxter/terrars-backblaze-b2)
- [digitalocean/digitalocean](https://github.com/andrewbaxter/terrars-digitalocean-digitalocean)
- [fly-apps/fly](https://github.com/andrewbaxter/terrars-fly-apps-fly)
- [hashicorp/aws](https://github.com/andrewbaxter/terrars-hashicorp-aws)
- [hashicorp/random](https://github.com/andrewbaxter/terrars-hashicorp-random)
- [kreuzwerker/docker](https://github.com/andrewbaxter/terrars-kreuzwerker-docker)

# Getting started

**Note**: There's a full, working example in [helloworld](helloworld).

1. Add `terrars` and pre-generated bindings such as [terrars-andrewbaxter-stripe](https://github.com/andrewbaxter/terrars-andrewbaxter-stripe) or else generate your own (see [Generation](#generation) below) to your project. Enable the features you want to use in the bindings.

2. Develop your code (ex: `build.rs`)

   Create a `Stack` and set up providers:

   ```rust
   let mut stack = &mut BuildStack{}.build();
   BuildProviderStripe {
       token: STRIPE_TOKEN,
   }.build(stack);
   ```

   The first provider instance for a provider type will be used by default for that provider's resources, so you don't need to bind it.

   Then create resources:

   ```rust
   let my_product = BuildProduct {
       name: "My Product".into(),
   }.build(stack);
   let my_price = BuildPrice {
       ...
   }.build(stack);
   my_price.set_product(my_product.id());
   ...
   ```

   Finally, write the stack out:

   ```rust
   fs::write("mystack.tf.json", &stack.serialize("state.json")?)?;
   ```

3. Call `terraform` as usual in the directory you generated `mystack.tf.json` in

   (`Stack` also has methods `run()` and `get_output()` to call `terraform` for you. You must have `terraform` in your path.)

# Generating bindings

While there are premade crates for some providers, you can generate code for new providers locally using `terrars-generate`.

1. Install the generate cli with `cargo install terrars`

2. Create a config file.
   As an example, to use `hashicorp/aws`, create a json file (ex: `terrars_aws.json`) with the specification of what you want to generate:

   ```json
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

   `tfschema/aws` must be an otherwise unused directory - it will be wiped when you genenerate the code. If `include` is missing or empty, this will generate everything (alternatively, you can use `exclude` to blacklist resources/datasources). Resources and datasources don't include the provider prefix (`aws_` in this example). Datasources start with `data_`.

3. Make sure you have `terraform` in your `PATH`. Run `cargo install terrars`, then `terrars-generate terrars_aws.json`.

4. The first time you do this, create a `src/bin/mydeploy/tfschema/mod.rs` file with this contents to root the generated provider:

   ```
   pub mod aws;
   ```

# General usage

## Definitions

There are `Build*` structs containing required parameters and a `build` method for most schema items (resources, stack, variables, outputs, etc). The `build` method registers the item in the `Stack` if applicable. Optional parameters can be set on the value returned from `build`.

## Expressions

Background: In Terraform, all fields regardless of type can be assigned a string template expression for values computed during stack application. Since all strings can potentially be templates, non-template strings must be escaped to avoid accidental interpolation.

How `terrars` handles it: When defining resources and calling methods, `String` and `&str` will be treated as non-template strings and appropriately escaped. To avoid the escaping, you can produce a `PrimExpr` object via `stack.str_expr` (to produce an expr that evaluates to a string) or `stack.expr` for other expression types. To produce the expression body you can use `format!()` as usual, but **note** - you must call `.raw()` on any `PrimExpr`s you use in the new expression to avoid double-antiescaping issues.

If Terraform gives you an error about something with the text `_TERRARS_SENTINEL*` it means you probably missed a `.raw()` call on that value (some expression was double-antiescaped).

As a rule of thumb

1. Converting from `expression` to `string`/`field` is OK. The expression gets turned into a sentinel value and interpolated during writing the template
2. Converting from `string`/`field` _with no sentinel values_ (literals, etc) to `expression` is OK.
3. Converting `string`/`field` _containing sentinel values_ -> `expression` is BAD. The sentinel replacement will happen twice and you'll have broken data. This can only happen if you convert an expression into a string and then back, so shouldn't happen often.

## For-each

Lists, sets, and record references have a `.map` method which takes care of all the different "for" methods in Terraform. Specifically

- Call `.map` and define a resource: does resource-level for-each (per Terraform limitations, this cannot be done on lists derived from other resources so has very limited use, you should probably just use a for loop)
- Call `.map` and define a block element: does block-level for-each
- Call `.map` and return an attribute reference: produces an attribute `for` expression

`.map` always produces a list reference, but this can be assgned to set fields as well. `.map_rec` is similar to `.map` but results in a record.

## Vecs and maps of primitives

There's two helper macros for generating vecs and maps of primitive values:

- `primvec![v, ...]` - creates a vec of primitive values, converting each value into a primitive if it is not. Use like `primvec!["stringone", "stringtwo"]` (easier than `vec!["stringone".into(), "stringtwo".into()]`).
- `primmap!{"k" = v, ...}` - creates a map of strings to primitive values, converting each value into a primitive if it is not. Same as above, performs automatic conversion.

# How it works

Terraform provides a method to output provider schemas as json. This tool uses that schema to generate structures that would output matching json Terraform stack files.

## Expressions/template strings/interpolation/escaping

Take as an example:

```rust
format!("{}{}", my_expr, verbatim_string))
```

This code would somehow need to escape the pattern and `verbatim_string`, while leaving `my_expr` unescaped, and the result would need to be treated as an "expression" to prevent escaping if it's used again in another `format!` or something. This applies to not just `format!` but serde serialization (json), other methods.

For now Terrars uses a simple (somewhat dirty) hack to avoid this. All expressions are put into a replacement table, and a sentinel string (ex: `_TERRARS_SENTINEL_99_`) is used instead. During final stack json serialization, the strings are escaped and then the original expression text is substituted back in , replacing the sentinel text.

This way, all normal string formatting methods should retain the expected expressions.

# Current limitations and warnings

- Not all Terraform features have been implemented

  The only one I'm aware of missing at the moment is resource Provisioning.

- `ignore_changes` takes strings rather than an enum

- No variable or output static type checking

  I'd like to add a derive macro for generating variables/outputs automatically from a structure at some point.

- Non-local deployment methods

  I think this is easy, but I haven't looked into it yet.

# The name

I originally called this `terrarust` but then I realized it sounded like terrorist so I decided to play it safe and chopped out the `u` `t` which stands for unreal tournament.
