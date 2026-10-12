import { forwardRef, type Ref } from "react";
import { TextInput as NativeInput, type TextInputProps } from "react-native";
import { UIInput } from "./primitives";
import { useThemeTokens } from "../theme/theme";

export const TextInput = forwardRef<
  NativeInput,
  TextInputProps & { className?: string }
>(function TextInput(
  { style, accessibilityLabel, placeholder, ...props },
  ref,
) {
  const tokens = useThemeTokens();
  return (
    <UIInput
      isDisabled={props.editable === false}
      style={{ width: "100%", minWidth: 0 }}
    >
      <UIInput.Input
        {...props}
        ref={ref as unknown as Ref<TextInputProps>}
        placeholder={placeholder}
        accessibilityLabel={accessibilityLabel ?? placeholder}
        aria-label={accessibilityLabel ?? placeholder}
        placeholderTextColor={tokens.colors.textSecondary}
        style={[
          {
            color: tokens.colors.textPrimary,
            backgroundColor: tokens.colors.surface,
            borderColor: tokens.colors.border,
            borderWidth: 1,
            borderRadius: tokens.radii.control,
            minHeight: 44,
            padding: 12,
            fontFamily: tokens.fonts.body,
            fontSize: 16,
          },
          style,
        ]}
      />
    </UIInput>
  );
});
