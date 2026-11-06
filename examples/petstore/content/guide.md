# Read the Petstore contract

This guide uses the unchanged OpenAPI document from the [`swagger-api/swagger-petstore` 3.0 release](https://github.com/swagger-api/swagger-petstore/tree/8f0dd286987880b4af7bce552aca3813166f3049/src/main/resources/openapi.yaml). Its source path is `src/main/resources/openapi.yaml`; the full Apache License 2.0 text is retained beside the fixture. The configured HTTPS URI is a source identifier, not a build-time download or a promise that a deployment is available.

## Follow a request into its response

Open [`getPetById`](../reference/getPetById/), the `GET /pet/{petId}` operation. Its path parameter `petId` is required and has the `int64` format. The operation declares a `200` response as either `application/json` or `application/xml`, each referring to `Pet`; it also documents `400`, `404`, and a default response.

The operation declares two alternative security requirements: an `api_key` API key in a header, or the `petstore_auth` implicit OAuth flow with `write:pets` and `read:pets` scopes. These are contract declarations. This example contains no credentials, login flow, or authenticated request runner.

## Inspect the returned model

Follow the response to [`Pet`](../reference/models/petstore/Pet/). Its required properties are `name` and `photoUrls`. `photoUrls` is an array of strings; the schema also refers to [`Category`](../reference/models/petstore/Category/) and an array of [`Tag`](../reference/models/petstore/Tag/) values. The `status` property lists `available`, `pending`, and `sold`.

Use the generated reference's local search for `photoUrls` to reach that field heading. The field link and fragment come from the source schema. The XML annotations remain source facts; they do not mean this reader implements an XML encoder.

## Compare other shapes and media

[`Order`](../reference/models/petstore/Order/) has a `shipDate` string with `date-time` format and a status enum of `placed`, `approved`, and `delivered`. The `getInventory` response is a map whose additional values are integers. `uploadFile` declares `application/octet-stream` and binary input. The contract also includes JSON, XML, and form media choices.

The current reader can present source-provided values and derive bounded reading examples for concrete JSON media. It does not synthesize XML, form, or binary wire payloads. A JSON display of a value under another media type is still a representation of the source value, not an encoding of that wire format. Examples and defaults are illustrative contract data, not validated application payloads. For instance, `User.password` has a source example; it is not a credential to reuse.
