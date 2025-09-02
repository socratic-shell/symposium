import Foundation

/// A Codable type that can represent arbitrary JSON data
enum JsonBlob: Codable, Equatable {
    struct PropertyKey: CodingKey, Hashable {
        var stringValue: String
        var intValue: Int?

        init(stringValue: String) {
            self.stringValue = stringValue
        }

        init(intValue: Int) {
            self.intValue = intValue
            self.stringValue = String(intValue)
        }
    }

    case object([PropertyKey: JsonBlob])
    case array([JsonBlob])
    case string(String)
    case number(Double)
    case boolean(Bool)
    case null
    
    func encode(to encoder: any Encoder) throws {
        switch self {
        case .object(let values):
            var container = encoder.container(keyedBy: PropertyKey.self)
            for (key, value) in values {
                try container.encode(value, forKey: key)
            }
        case .array(let values):
            var container = encoder.unkeyedContainer()
            for value in values {
                try container.encode(value)
            }
        case .string(let value):
            var container = encoder.singleValueContainer()
            try container.encode(value)
        case .number(let value):
            var container = encoder.singleValueContainer()
            try container.encode(value)
        case .boolean(let value):
            var container = encoder.singleValueContainer()
            try container.encode(value)
        case .null:
            var container = encoder.singleValueContainer()
            try container.encodeNil()
        }
    }
    
    init(from decoder: any Decoder) throws {
        if let container = try? decoder.container(keyedBy: PropertyKey.self) {
            var values: [PropertyKey: JsonBlob] = [:]
            for key in container.allKeys {
                values[key] = try container.decode(JsonBlob.self, forKey: key)
            }
            self = .object(values)
        } else if var container = try? decoder.unkeyedContainer() {
            var values: [JsonBlob] = []
            while !container.isAtEnd {
                values.append(try container.decode(JsonBlob.self))
            }
            self = .array(values)
        } else {
            let container = try decoder.singleValueContainer()
            if let value = try? container.decode(String.self) {
                self = .string(value)
            } else if let value = try? container.decode(Double.self) {
                self = .number(value)
            } else if let value = try? container.decode(Bool.self) {
                self = .boolean(value)
            } else {
                guard container.decodeNil() else {
                    throw DecodingError.dataCorruptedError(in: container, debugDescription: "Data unrecognizable")
                }
                self = .null
            }
        }
    }
}
