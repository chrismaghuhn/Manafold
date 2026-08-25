"""Experimental internal client package for the temporary M2 semantic adapter.

Non-public (leading underscore); deliberately absent from ``mtgml.__all__``.
Stdlib-only; the Rust adapter remains the sole semantic authority.
"""

from .client import AdapterPlayerClient, SyntheticEnvironmentClient
from .process import RestrictedPlayerTransport, SubprocessTransport
from .protocol import AdapterError

__all__ = [
    "AdapterError",
    "AdapterPlayerClient",
    "RestrictedPlayerTransport",
    "SubprocessTransport",
    "SyntheticEnvironmentClient",
]
