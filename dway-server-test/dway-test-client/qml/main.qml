import QtQuick 2.12
import QtQuick.Controls 2.12
import QtQuick.Window 2.12

import com.dway_srver_test.qt5_client 1.0

Window {
    height: 480
    title: qsTr("Hello World")
    visible: true
    width: 640

    UiObject {
        id: myObject
        number: 0
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        Label {
            text: qsTr("Number: %1").arg(myObject.number)
        }

        Button {
            text: qsTr("exit")

            onClicked: myObject.button_exit()
        }
    }
}
