Option Explicit

Dim fso, shell, msiDir, sourceDir, targetDir, modelsSource, modelsTarget

Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")

' Get the MSI directory from the MSIDIR property passed by WiX
msiDir = Session.Property("MSIDIR")

' If MSIDIR is empty, try to get it from the current directory
If msiDir = "" Then
    msiDir = fso.GetParentFolderName(WScript.ScriptFullName)
End If

' Get the installation target directory from INSTALLDIR property
targetDir = Session.Property("INSTALLDIR")

' Define source and target paths for the models folder
modelsSource = fso.BuildPath(msiDir, "models")
modelsTarget = fso.BuildPath(targetDir, "models")

' Check if source models folder exists
If fso.FolderExists(modelsSource) Then
    ' If target models folder already exists, delete it first
    If fso.FolderExists(modelsTarget) Then
        fso.DeleteFolder modelsTarget, True
    End If

    ' Copy the models folder
    fso.CopyFolder modelsSource, modelsTarget, True

    ' Return success
    Session.Property("COPYMODELS_RESULT") = "Success"
Else
    ' Models folder not found - log error but don't fail installation
    Session.Property("COPYMODELS_RESULT") = "Source folder not found: " & modelsSource
End If
