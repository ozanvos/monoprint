// Verify with the public key embedded in the shipped app, independent of the
// private key selected by generate_appcast. Requires the macOS packaging host.
import CryptoKit
import Foundation

guard CommandLine.arguments.count == 4,
      let signature = Data(base64Encoded: CommandLine.arguments[2]),
      let publicKey = Data(base64Encoded: CommandLine.arguments[3]) else {
    fputs("Usage: verify-sparkle-signature.swift ARCHIVE SIGNATURE PUBLIC_KEY\n", stderr)
    exit(2)
}
do {
    let key = try Curve25519.Signing.PublicKey(rawRepresentation: publicKey)
    let bytes = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
    guard key.isValidSignature(signature, for: bytes) else {
        fputs("Update signature does not match the public key embedded in the app\n", stderr)
        exit(1)
    }
} catch {
    fputs("Cannot verify update signature: \(error)\n", stderr)
    exit(1)
}
