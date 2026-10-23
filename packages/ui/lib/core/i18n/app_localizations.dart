import "package:flutter/foundation.dart";
import "package:flutter/widgets.dart";

import "app_strings.dart";

/// Resolves [AppStrings] for the active locale.
class AppLocalizations {
    const AppLocalizations({required this.locale, required this.strings});

    /// Locale resolved for this instance.
    final Locale locale;

    /// Strings for [locale].
    final AppStrings strings;

    /// Locales the interface ships with.
    static const List<Locale> supportedLocales = [
        Locale("en", "US"),
        Locale("pt", "BR"),
    ];

    /// Delegate registered by the application.
    static const LocalizationsDelegate<AppLocalizations> delegate =
            _AppLocalizationsDelegate();

    /// Returns the closest localization for [context].
    static AppLocalizations of(BuildContext context) {
        final value = Localizations.of<AppLocalizations>(context, AppLocalizations);
        assert(value != null, "AppLocalizations is not available in the widget tree.");
        return value!;
    }

    /// Picks the string set for [locale], falling back to English.
    static AppStrings resolve(Locale locale) {
        return locale.languageCode == "pt" ? AppStrings.ptBr : AppStrings.en;
    }
}

class _AppLocalizationsDelegate
        extends LocalizationsDelegate<AppLocalizations> {
    const _AppLocalizationsDelegate();

    @override
    bool isSupported(Locale locale) {
        return AppLocalizations.supportedLocales.any(
            (supported) => supported.languageCode == locale.languageCode,
        );
    }

    @override
    Future<AppLocalizations> load(Locale locale) {
        return SynchronousFuture(
            AppLocalizations(
                locale: locale,
                strings: AppLocalizations.resolve(locale),
            ),
        );
    }

    @override
    bool shouldReload(covariant LocalizationsDelegate<AppLocalizations> old) {
        return false;
    }
}
