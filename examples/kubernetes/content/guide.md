# Reading this specimen

The reference is built from the Kubernetes API Extensions OpenAPI document at a pinned public source revision. The original JSON is included unchanged with its Apache License 2.0 notice. The package documentation records fixture provenance and measured coverage.

## Follow a CRD read response

Start at [`readApiextensionsV1CustomResourceDefinition`](../reference/readApiextensionsV1CustomResourceDefinition/). Its `200` response offers several source media types, including `application/json`, and refers to [CustomResourceDefinition](../reference/models/apiextensions/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.CustomResourceDefinition/). That model requires `spec`; follow it to [CustomResourceDefinitionSpec](../reference/models/apiextensions/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.CustomResourceDefinitionSpec/). When choosing a response representation in the reader, select `application/json` for the JSON view; the first declared CBOR type does not make the reader encode a JSON value as CBOR. These links follow the documented schema, not a request to a Kubernetes cluster.

The contract does not declare a server or security requirement. The generator’s reserved `https://api.example.invalid` host is an illustrative placeholder; no request is sent, and this fixture does not establish a deployment endpoint or authentication method.

## Media and examples

The reader preserves the original media labels. It can display JSON examples and derive bounded JSON reading aids for concrete JSON media, including concrete `+json` subtypes. A source-provided value under another media type remains visible as a JSON representation of that source value; it is not encoded as CBOR, protobuf, YAML, or another wire format. If no source value is present for unsupported media, the reader reports that it cannot encode a schema into that media. It does not synthesize XML. Labels such as `application/json;stream=watch` remain source labels: the reader does not validate stream framing.

## Schema facts

The concise reader follows supported OpenAPI schema positions and makes recursion, map expansion, and other bounded stops visible. Names such as `$ref`, `oneOf`, and `allOf` inside the `JSONSchemaProps` data model are displayed as fields; they are not interpreted as schema keywords there. Bounded examples are reading aids, not Kubernetes or JSON Schema payload validation.
