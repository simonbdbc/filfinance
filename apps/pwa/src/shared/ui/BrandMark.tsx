import {
    Image,
    Text as NativeText,
    View,
    type ImageSourcePropType,
} from "react-native";
import { useThemeTokens } from "@shared/theme/theme";
import { Text, headingStyle } from "./Text";

const brandSource =
    require("../../../assets/favicon.png") as ImageSourcePropType;

/** Shared brand signature; the release label is descriptive, never interactive. */
export function BrandMark({
    variant = "mobile",
}: {
    variant?: "desktop" | "mobile" | "auth";
}) {
    const t = useThemeTokens();
    const stacked = variant === "auth";
    const size = stacked ? 56 : variant === "desktop" ? 44 : 32;
    const titleSize = stacked ? 32 : variant === "desktop" ? 28 : 24;

    return (
        <View
            testID="brand-mark"
            style={{
                flexDirection: stacked ? "column" : "row",
                alignItems: "center",
                gap: stacked ? 30 : 12,
                paddingTop: stacked ? 0 : 18,
                maxWidth: "100%",
                flexShrink: 0,
            }}
        >
            <Image
                accessibilityIgnoresInvertColors
                accessibilityLabel="Fil Finance"
                accessibilityRole="image"
                resizeMode="contain"
                source={brandSource}
                style={{ height: size, width: size }}
            />
            <View style={{ position: "relative" }}>
                <Text testID="brand-name" style={headingStyle(t, titleSize)}>
                    {stacked ? "Fil Finance" : "fil finance"}
                </Text>
                <View
                    testID="brand-beta"
                    style={{
                        position: "absolute",
                        top: -6,
                        right: -21,
                        borderRadius: t.radii.control,
                        borderWidth: 1,
                        borderColor: t.colors.borderSoft,
                        backgroundColor: t.colors.background,
                        paddingHorizontal: 4,
                        paddingVertical: 0,
                    }}
                >
                    {/* Compact brand annotation, independent of the 14px body-text minimum. */}
                    <NativeText
                        style={{
                            fontFamily: t.fonts.body,
                            color: t.colors.textPrimary,
                            fontSize: 8,
                            lineHeight: 14,
                            fontWeight: "600",
                        }}
                    >
                        Bêta
                    </NativeText>
                </View>
            </View>
        </View>
    );
}
