// Round-trips every sample against the Kotlinx output with `unknown_variants`. Unknown
// payloads must decode, and a top-level unknown must land on the generated `Unknown` variant.
package dev.specta.kotlincheck

import kotlinx.serialization.json.JsonElement

private fun decodeSample(name: String, element: JsonElement): Any = when (name) {
    "ApiError" -> decode(ApiError.serializer(), element)
    "AuthApiError" -> decode(AuthApiError.serializer(), element)
    "SignInRemovalCause" -> decode(SignInRemovalCause.serializer(), element)
    "RouteSessionType" -> decode(RouteSessionType.serializer(), element)
    "InternallyTagged" -> decode(InternallyTagged.serializer(), element)
    "ExternalEvent" -> decode(ExternalEvent.serializer(), element)
    "Settings" -> decode(Settings.serializer(), element)
    else -> error("unknown sample type: $name")
}

private fun encodeSample(name: String, value: Any): JsonElement = when (name) {
    "ApiError" -> encode(ApiError.serializer(), value as ApiError)
    "AuthApiError" -> encode(AuthApiError.serializer(), value as AuthApiError)
    "SignInRemovalCause" -> encode(SignInRemovalCause.serializer(), value as SignInRemovalCause)
    "RouteSessionType" -> encode(RouteSessionType.serializer(), value as RouteSessionType)
    "InternallyTagged" -> encode(InternallyTagged.serializer(), value as InternallyTagged)
    "ExternalEvent" -> encode(ExternalEvent.serializer(), value as ExternalEvent)
    "Settings" -> encode(Settings.serializer(), value as Settings)
    else -> error("unknown sample type: $name")
}

private fun isFallback(name: String, value: Any): Boolean = when (name) {
    "ApiError" -> value is ApiError.Unknown
    "AuthApiError" -> value is AuthApiError.Unknown
    "SignInRemovalCause" -> value is SignInRemovalCause.Unknown
    "RouteSessionType" -> value is RouteSessionType.Unknown
    "InternallyTagged" -> value is InternallyTagged.Unknown
    "ExternalEvent" -> value is ExternalEvent.Unknown
    else -> false
}

fun main(args: Array<String>) {
    for (sample in readSamples(args.single())) {
        val decoded = decodeSample(sample.name, sample.value)
        if (sample.fallback) {
            Check.isTrue(sample.name, isFallback(sample.name, decoded), "did not decode to the Unknown variant")
        }
        Check.equal(sample.name, sample.value, encodeSample(sample.name, decoded))
    }
    Check.finish()
}
