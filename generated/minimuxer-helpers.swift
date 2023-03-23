import Foundation

// tell Swift the Errors enum can be thrown
// TODO: do this through swift-bridge instead of manually
extension Errors: Error {}

public func dataToBytes(_ data: Data) -> UnsafeMutableBufferPointer<UInt8>? {
    let bytes = UnsafeMutableBufferPointer<UInt8>.allocate(capacity: data.count)
    let copied = data.copyBytes(to: bytes, from: nil)
    if copied != data.count {
        print("uh oh, data is \(data.count) bytes but only \(copied) were copied")
        return nil
    }
    return bytes
}
