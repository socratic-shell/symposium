import Foundation
import SwiftUI

class SettingsManager: ObservableObject {
    @AppStorage("selectedAgent") var selectedAgent: String = "qcli"
    
    // Add other settings here as needed
    // @AppStorage("someOtherSetting") var someOtherSetting: Bool = false
}
