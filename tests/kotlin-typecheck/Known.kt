// Round-trips the known samples against the Kotlinx output without `unknown_variants`.
// An unknown payload must be rejected there instead of decoding to a fallback.
package dev.specta.kotlincheck

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.JsonElement

private fun decodeAndEncode(name: String, element: JsonElement): JsonElement = when (name) {
    "ApiError" -> roundTrip(ApiError.serializer(), element)
    "AuthApiError" -> roundTrip(AuthApiError.serializer(), element)
    "SignInRemovalCause" -> roundTrip(SignInRemovalCause.serializer(), element)
    "RouteSessionType" -> roundTrip(RouteSessionType.serializer(), element)
    "InternallyTagged" -> roundTrip(InternallyTagged.serializer(), element)
    "ExternalEvent" -> roundTrip(ExternalEvent.serializer(), element)
    "Settings" -> roundTrip(Settings.serializer(), element)
    else -> error("unknown sample type: $name")
}

fun main(args: Array<String>) {
    for (sample in readSamples(args.single())) {
        if (!sample.unknown) {
            Check.equal(sample.name, sample.value, decodeAndEncode(sample.name, sample.value))
            continue
        }

        val rejected = try {
            decodeAndEncode(sample.name, sample.value)
            false
        } catch (_: SerializationException) {
            true
        }
        Check.isTrue(sample.name, rejected, "decoded an unknown payload without unknown_variants")
    }
    Check.finish()
}
