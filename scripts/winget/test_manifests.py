import unittest

from manifests import IDENTIFIER, manifests

SUMS = "\n".join(f"{c * 64}  Bobine-Deploy-Setup-0.1.0-{arch}.exe" for c, arch in (("a", "x64"), ("b", "arm64")))


class ManifestTests(unittest.TestCase):
    def test_four_documents_with_schema_header(self):
        documents = manifests(SUMS, "v0.1.0")
        self.assertEqual(len(documents), 4)
        for content in documents.values():
            self.assertTrue(content.startswith("# yaml-language-server: $schema=https://aka.ms/winget-manifest."))
            self.assertIn("PackageVersion: 0.1.0\n", content)

    def test_installers_point_at_the_release(self):
        installer = manifests(SUMS, "v0.1.0")[f"{IDENTIFIER}.installer.yaml"]
        self.assertIn("InstallerType: nullsoft\nScope: user\n", installer)
        self.assertIn("releases/download/v0.1.0/Bobine-Deploy-Setup-0.1.0-arm64.exe", installer)
        self.assertIn("InstallerSha256: " + "B" * 64, installer)

    def test_descriptions_are_quoted(self):
        locale = manifests(SUMS, "v0.1.0")[f"{IDENTIFIER}.locale.fr-FR.yaml"]
        self.assertIn("ShortDescription: 'Installer Bobine", locale)

    def test_missing_installer_or_bad_tag_is_refused(self):
        with self.assertRaises(SystemExit):
            manifests(SUMS.splitlines()[0], "v0.1.0")
        with self.assertRaises(SystemExit):
            manifests(SUMS, "latest")


if __name__ == "__main__":
    unittest.main()
