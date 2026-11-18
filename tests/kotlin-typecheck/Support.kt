package dev.specta.kotlincheck

import java.io.File
import kotlin.system.exitProcess
import kotlinx.serialization.KSerializer
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

internal val json = Json

internal data class Sample(
    val name: String,
    val unknown: Boolean,
    val fallback: Boolean,
    val value: JsonElement,
)

internal fun readSamples(path: String): List<Sample> =
    json.parseToJsonElement(File(path).readText()).jsonArray.map { entry ->
        val fields = entry.jsonObject
        Sample(
            name = fields.getValue("name").jsonPrimitive.content,
            unknown = fields.getValue("unknown").jsonPrimitive.content == "true",
            fallback = fields.getValue("fallback").jsonPrimitive.content == "true",
            value = fields.getValue("value"),
        )
    }

internal fun <T> decode(serializer: KSerializer<T>, element: JsonElement): T =
    json.decodeFromJsonElement(serializer, element)

internal fun <T> encode(serializer: KSerializer<T>, value: T): JsonElement =
    json.encodeToJsonElement(serializer, value)

internal fun <T> roundTrip(serializer: KSerializer<T>, element: JsonElement): JsonElement =
    encode(serializer, decode(serializer, element))

internal object Check {
    private var passed = 0
    private var failures = 0

    fun equal(name: String, expected: JsonElement, actual: JsonElement) {
        if (expected == actual) {
            passed++
        } else {
            failures++
            System.err.println("$name: round-trip produced $actual, expected $expected")
        }
    }

    fun isTrue(name: String, condition: Boolean, message: String) {
        if (condition) {
            passed++
        } else {
            failures++
            System.err.println("$name: $message")
        }
    }

    fun finish() {
        if (failures != 0) {
            System.err.println("kotlin-typecheck: $failures of ${passed + failures} checks failed")
            exitProcess(1)
        }
        println("kotlin-typecheck: $passed checks passed")
    }
}
