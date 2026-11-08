/// A value being loaded: the latest data (kept while reloading so numbers don't flash away), a
/// loading flag and the last error. Deliberately tiny and independent of Riverpod's AsyncValue.
class Loadable<T> {
  const Loadable({this.data, this.loading = false, this.error});

  final T? data;
  final bool loading;
  final Object? error;

  bool get hasData => data != null;
  bool get failed => error != null && !loading;
}
